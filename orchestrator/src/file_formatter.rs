use anyhow::{bail, Context};
use encoding_rs::Encoding;
use log::*;
use std::{
    borrow::Cow,
    fmt::Display,
    fs::{File, OpenOptions},
    io::{self, IsTerminal, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use glob::glob;
use pasfmt_core::formatter::Formatter;
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::ErrHandler;

struct DecodedFile<'a> {
    bom: Option<&'a [u8]>,
    contents: Cow<'a, str>,
    encoding: &'static Encoding,
}

pub struct FileFormatter {
    formatter: Formatter,
    encoding: &'static encoding_rs::Encoding,
}
impl FileFormatter {
    pub fn new(formatter: Formatter, encoding: &'static encoding_rs::Encoding) -> Self {
        FileFormatter {
            formatter,
            encoding,
        }
    }

    fn expand_paths<S: AsRef<str>>(&self, paths: &[S]) -> Vec<Result<PathBuf, anyhow::Error>> {
        let mut expanded_paths = vec![];
        paths.iter().map(AsRef::as_ref).for_each(|path_str| {
            let path = Path::new(path_str);
            match path.metadata() {
                Ok(metadata) if metadata.is_dir() => {
                    expanded_paths.extend(WalkDir::new(path_str).into_iter().filter_map(|entry| {
                        match entry {
                            Ok(entry) => {
                                let file_path = entry.path();
                                match formattable_file_path(file_path) {
                                    true => Some(Ok(file_path.to_path_buf())),
                                    false => None,
                                }
                            }
                            Err(e) => Some(Err(e.into())),
                        }
                    }));
                }
                _ if path_str.contains('*') => {
                    match glob(path_str)
                        .with_context(|| format!("invalid glob expression `{path_str}`"))
                    {
                        Err(e) => expanded_paths.push(Err(e)),
                        Ok(glob) => {
                            expanded_paths.extend(glob.map(|entry| entry.map_err(|e| e.into())))
                        }
                    }
                }
                _ => {
                    expanded_paths.push(Ok(path.to_path_buf()));
                }
            }
        });

        expanded_paths
    }

    fn decode_file<'a>(
        &self,
        file: &mut impl Read,
        name: impl Display,
        buf: &'a mut Vec<u8>,
    ) -> anyhow::Result<DecodedFile<'a>> {
        file.read_to_end(buf)?;

        let (encoding, (bom, contents)) = match Encoding::for_bom(buf) {
            Some((encoding, bom_length)) => {
                (encoding, (Some(&buf[..bom_length]), &buf[bom_length..]))
            }
            None => (self.encoding, (None, &buf[..])),
        };
        let (contents, replacements) = encoding.decode_without_bom_handling(contents);

        if replacements {
            bail!(
                "File '{}' has malformed sequences (in encoding '{}'{})",
                name,
                encoding.name(),
                bom.map(|_| " - inferred from BOM").unwrap_or(""),
            );
        }

        Ok(DecodedFile {
            bom,
            contents,
            encoding,
        })
    }

    fn exec_format<S, T, E>(
        &self,
        paths: &[S],
        mut open_options: OpenOptions,
        result_operation: T,
        error_handler: E,
    ) where
        S: AsRef<str>,
        E: ErrHandler,
        T: Fn(&mut File, &Path, &DecodedFile, &str) -> anyhow::Result<()> + Sync,
    {
        open_options.read(true);

        self.expand_paths(paths)
            .into_par_iter()
            .map_init(
                || (Vec::<u8>::new(), String::new()),
                |(input_buf, output_buf), file_path| {
                    input_buf.clear();
                    output_buf.clear();

                    let file_path = file_path?;
                    let mut file = open_options
                        .open(&file_path)
                        .with_context(|| format!("Failed to open '{}'", file_path.display()))?;

                    let decoded_file = self
                        .decode_file(&mut file, file_path.display(), input_buf)
                        .with_context(|| format!("Failed to read '{}'", file_path.display()))?;

                    self.formatter
                        .format_into_buf(&decoded_file.contents, output_buf);
                    result_operation(&mut file, &file_path, &decoded_file, output_buf)
                },
            )
            .for_each(|res| {
                if let Err(e) = res {
                    error_handler(e);
                };
            });
    }

    pub(crate) fn format_files<S: AsRef<str>>(&self, paths: &[S], error_handler: impl ErrHandler) {
        self.exec_format(
            paths,
            OpenOptions::new().write(true).to_owned(),
            |file, file_path, decoded_file, formatted_output| {
                if decoded_file.contents.eq(&formatted_output) {
                    debug!(
                        "Skipping writing to '{}' because it is already formatted.",
                        file_path.display()
                    );
                    return Ok(());
                }
                file.seek(SeekFrom::Start(0)).with_context(|| {
                    format!("Failed to seek to start of file: '{}'", file_path.display())
                })?;
                let new_len = Self::write_out(file, decoded_file, formatted_output)
                    .with_context(|| format!("Failed to write to '{}'", file_path.display()))?;
                file.set_len(new_len).with_context(|| {
                    format!("Failed to set file length: '{}'", file_path.display())
                })?;
                Ok(())
            },
            error_handler,
        );
    }

    fn encode_utf16(data: &str, u16_encoder: impl Fn(u16) -> [u8; 2]) -> Vec<u8> {
        // 2 * utf8_len is a pretty good approximation for utf16_len in most cases.
        let mut out = Vec::with_capacity(2 * data.len());
        out.extend(data.encode_utf16().flat_map(u16_encoder));
        out
    }

    fn encode_utf16be(data: &str) -> Vec<u8> {
        Self::encode_utf16(data, u16::to_be_bytes)
    }
    fn encode_utf16le(data: &str) -> Vec<u8> {
        Self::encode_utf16(data, u16::to_le_bytes)
    }

    fn encode<'a>(encoding: &'static Encoding, data: &'a str) -> io::Result<Cow<'a, [u8]>> {
        // encoding_rs doesn't support encoding to UTF16, so we do it ourselves.
        if encoding.output_encoding() == encoding {
            let (encoded_data, _encoding_used, replacements) = encoding.encode(data);
            if replacements {
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Formatting result contains data that cannot be encoded as {}",
                        encoding.name()
                    ),
                ))
            } else {
                Ok(encoded_data)
            }
        } else if encoding == encoding_rs::UTF_16BE {
            Ok(Cow::Owned(Self::encode_utf16be(data)))
        } else if encoding == encoding_rs::UTF_16LE {
            Ok(Cow::Owned(Self::encode_utf16le(data)))
        } else {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("No encoder for encoding {}", encoding.name()),
            ))
        }
    }

    fn write_out(
        write: &mut impl Write,
        decoded: &DecodedFile,
        data: &str,
    ) -> std::io::Result<u64> {
        let encoded_output = Self::encode(decoded.encoding, data)?;

        let mut len = 0;
        if let Some(bom) = decoded.bom {
            write.write_all(bom)?;
            len += bom.len();
        }
        write.write_all(&encoded_output)?;
        len += encoded_output.len();

        Ok(len as u64)
    }

    fn decode_stdin<'a>(&self, buf: &'a mut Vec<u8>) -> anyhow::Result<DecodedFile<'a>> {
        if std::io::stdin().is_terminal() {
            eprintln!("waiting for stdin...");
        }
        let mut stdin = std::io::stdin().lock();

        self.decode_file(&mut stdin, "<stdin>", buf)
            .context("Failed to read from stdin")
    }

    pub(crate) fn format_stdin_to_stdout(&self, error_handler: impl ErrHandler) {
        let inner = || {
            let mut buf = vec![];
            let decoded_stdin = self.decode_stdin(&mut buf)?;
            let formatted_input = self.formatter.format(&decoded_stdin.contents);
            Self::write_out(&mut std::io::stdout(), &decoded_stdin, &formatted_input)
                .context("Failed to write to stdout")?;
            Ok(())
        };

        if let Err(e) = inner() {
            error_handler(e);
        }
    }

    pub(crate) fn format_files_to_stdout<S: AsRef<str>>(
        &self,
        paths: &[S],
        error_handler: impl ErrHandler,
    ) {
        self.exec_format(
            paths,
            OpenOptions::new(),
            |_, file_path, _, formatted_output| {
                // Append a newline to the formatted output deliberately because makes it more
                // human-readable and no less machine-readable.
                println!("{}:\n{}", file_path.display(), formatted_output);
                Ok(())
            },
            error_handler,
        );
    }

    fn check_formatting(input: &str, output: &str, path: impl Display) -> anyhow::Result<()> {
        if input != output {
            bail!("CHECK: '{}' has incorrect formatting", path);
        }
        Ok(())
    }

    pub(crate) fn check_files<S: AsRef<str>>(&self, paths: &[S], error_handler: impl ErrHandler) {
        self.exec_format(
            paths,
            OpenOptions::new(),
            |_, file_path, decoded_file, formatted_output| {
                Self::check_formatting(
                    &decoded_file.contents,
                    formatted_output,
                    file_path.display(),
                )
            },
            error_handler,
        );
    }

    pub(crate) fn check_stdin(&self, error_handler: impl ErrHandler) {
        let inner = || {
            let mut buf = vec![];
            let decoded_stdin = self.decode_stdin(&mut buf)?;
            let formatted_input = self.formatter.format(&decoded_stdin.contents);
            Self::check_formatting(&decoded_stdin.contents, &formatted_input, "<stdin>")
        };

        if let Err(e) = inner() {
            error_handler(e);
        }
    }
}

fn formattable_file_path(path: &Path) -> bool {
    match path.extension() {
        Some(ext) => {
            ext.eq_ignore_ascii_case("pas")
                || ext.eq_ignore_ascii_case("dpr")
                || ext.eq_ignore_ascii_case("dpk")
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use yare::parameterized;

    use crate::file_formatter::formattable_file_path;
    use std::path::PathBuf;

    #[parameterized(
        pas_lower = {"a.pas"},
        pas_upper = {"a.PAS"},
        pas_mixed = {"a.Pas"},
        dpr_lower = {"b.dpr"},
        dpr_upper = {"b.DPR"},
        dpr_mixed = {"b.Dpr"},
        dpk_lower = {"c.dpk"},
        dpk_upper = {"c.DPK"},
        dpk_mixed = {"c.Dpk"},
    )]
    fn formattable_file_paths(path: &str) {
        assert!(formattable_file_path(&PathBuf::from(path)));
    }

    #[parameterized(
        no_dot_pas = {"pas"},
        starts_pas = {"a.pas1"},
        ends_pas = {"a.unpas"},
        contains_dot_pas = {"a.pas.x"},
        no_dot_dpr = {"dpr"},
        starts_dpr = {"a.dpr1"},
        ends_dpr = {"a.undpr"},
        contains_dot_dpr = {"a.dpr.x"},
        no_dot_dpk = {"dpk"},
        starts_dpk = {"a.dpk1"},
        ends_dpk = {"a.undpk"},
        contains_dot_dpk = {"a.dpk.x"},
        only_dot_pas = {".pas"},
        only_dot_dpr = {".dpr"},
        only_dot_dpk = {".dpk"},
    )]
    fn non_formattable_file_paths(path: &str) {
        assert!(!formattable_file_path(&PathBuf::from(path)));
    }
}

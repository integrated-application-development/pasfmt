#![deny(clippy::enum_glob_use)]

use encoding_rs::Encoding;
use pasfmt_core::prelude::*;
use pasfmt_orchestrator::predule::*;
use serde::Deserialize;

#[cfg(windows)]
fn get_windows_default_encoding() -> &'static Encoding {
    fn inner() -> &'static Encoding {
        // SAFETY: yes it's a foreign function, but it's a simple one from the WinAPI that we
        // can assume to be safe.
        let ansi_codepage = unsafe { windows_sys::Win32::Globalization::GetACP() };
        let encoding = ansi_codepage
            .try_into()
            .ok()
            .and_then(|cp: u16| codepage::to_encoding(cp))
            .unwrap_or_else(|| {
                log::warn!(
                    "failed to convert system codepage {} to encoding, defaulting to UTF-8",
                    ansi_codepage
                );
                encoding_rs::UTF_8
            });
        log::debug!("encoding from system ANSI codepage: {}", encoding.name());
        encoding
    }

    use std::sync::LazyLock;
    static WINDOWS_DEFAULT_ENCODING: LazyLock<&'static Encoding> = LazyLock::new(inner);
    *WINDOWS_DEFAULT_ENCODING
}

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug, Clone, Copy, Default)]
#[serde(rename_all = "lowercase")]
enum LineEnding {
    #[serde(alias = "CRLF")]
    Crlf,
    #[serde(alias = "LF")]
    Lf,
    #[default]
    Native,
}

impl From<LineEnding> for pasfmt_core::lang::LineEnding {
    fn from(value: LineEnding) -> Self {
        use pasfmt_core::lang::LineEnding as CoreLineEnding;

        match value {
            LineEnding::Crlf => CoreLineEnding::Crlf,
            LineEnding::Lf => CoreLineEnding::Lf,
            #[cfg(windows)]
            LineEnding::Native => CoreLineEnding::Crlf,
            #[cfg(not(windows))]
            LineEnding::Native => CoreLineEnding::Lf,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
enum InternalEncoding {
    #[default]
    Native,
    Named(&'static Encoding),
}

impl InternalEncoding {
    const NATIVE_ENCODING_NAME: &str = "native";
}

impl From<InternalEncoding> for &'static Encoding {
    fn from(value: InternalEncoding) -> Self {
        match value {
            InternalEncoding::Named(encoding) => encoding,
            #[cfg(windows)]
            InternalEncoding::Native => get_windows_default_encoding(),
            #[cfg(not(windows))]
            InternalEncoding::Native => encoding_rs::UTF_8,
        }
    }
}

#[cfg(feature = "__demo")]
impl serde::Serialize for InternalEncoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(match self {
            InternalEncoding::Native => Self::NATIVE_ENCODING_NAME,
            InternalEncoding::Named(encoding) => encoding.name(),
        })
    }
}

struct InternalEncodingVisitor;
impl serde::de::Visitor<'_> for InternalEncodingVisitor {
    type Value = InternalEncoding;

    fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
        formatter.write_fmt(format_args!(
            "\"{}\" or a valid encoding label",
            InternalEncoding::NATIVE_ENCODING_NAME
        ))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if value.eq_ignore_ascii_case(InternalEncoding::NATIVE_ENCODING_NAME) {
            Ok(InternalEncoding::Native)
        } else if let Some(enc) = Encoding::for_label(value.as_bytes()) {
            Ok(InternalEncoding::Named(enc))
        } else {
            Err(E::invalid_value(serde::de::Unexpected::Str(value), &self))
        }
    }
}

impl<'de> Deserialize<'de> for InternalEncoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(InternalEncodingVisitor)
    }
}

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Debug, Clone, Copy, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum BeginStyle {
    #[default]
    Auto,
    #[allow(non_camel_case_types)]
    Always_Wrap,
}

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
#[serde(default)]
pub struct FormattingConfig {
    wrap_column: u32,
    begin_style: BeginStyle,

    encoding: InternalEncoding,

    use_tabs: bool,
    tab_width: u8,
    continuation_indents: u8,
    line_ending: LineEnding,
}

impl FormattingConfig {
    #[cfg(feature = "__demo")]
    pub fn max_line_length(&self) -> u32 {
        self.wrap_column
    }
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            line_ending: LineEnding::default(),
            use_tabs: false,
            tab_width: 2,
            continuation_indents: 2,
            encoding: InternalEncoding::default(),
            wrap_column: 120,
            begin_style: BeginStyle::default(),
        }
    }
}

impl From<&FormattingConfig> for ReconstructionSettings {
    fn from(val: &FormattingConfig) -> Self {
        // The core measures indents and continuations in counts of spaces or tabs, whereas here
        // it's measured as a count of 'indentations', so we have to convert.
        let (indent_width, continuation_width, tab) = if val.use_tabs {
            (1, val.continuation_indents, TabKind::Hard)
        } else {
            (
                val.tab_width,
                val.continuation_indents.saturating_mul(val.tab_width),
                TabKind::Soft,
            )
        };

        ReconstructionSettings::new(
            val.line_ending.into(),
            tab,
            indent_width,
            continuation_width,
        )
    }
}

impl From<&FormattingConfig> for OptimisingLineFormatterSettings {
    fn from(value: &FormattingConfig) -> Self {
        Self {
            max_line_length: value.wrap_column,
            iteration_max: 20_000,
            break_before_begin: matches!(value.begin_style, BeginStyle::Always_Wrap),
        }
    }
}

impl Configuration for FormattingConfig {
    fn docs() -> impl IntoIterator<Item = ConfigItem> {
        let defaults = Self::default();
        vec![
            ConfigItem {
                name: "wrap_column",
                description: "Target line length before wrapping",
                hint: "<unsigned integer>",
                default: defaults.wrap_column.to_string(),
            },
            ConfigItem {
                name: "begin_style",
                description: "\
Places the `begin` after control flow statements (e.g. `if`).
If \"always_wrap\", the `begin` will always be placed on the next line
at the same indentation as the statement it is within.\
                    ",
                hint: "[ auto | always_wrap ]",
                default: format!("{:?}", defaults.begin_style).to_lowercase(),
            },
            ConfigItem {
                name: "encoding",
                description: "\
The encoding to use when reading and writing files.
If \"native\":
  * on Windows, the system ANSI codepage is used
  * otherwise, UTF-8 is used

In all cases a detected BOM will override the configured encoding.\
                    ",
                hint: "native | <NAME>",
                default: format!("{:?}", defaults.encoding).to_lowercase(),
            },
            ConfigItem {
                name: "use_tabs",
                description: "Use tab characters for indentation",
                hint: "<boolean>",
                default: defaults.use_tabs.to_string(),
            },
            ConfigItem {
                name: "tab_width",
                description: "Number of spaces per indentation (ignored if use_tabs=true)",
                hint: "<unsigned integer>",
                default: defaults.tab_width.to_string(),
            },
            ConfigItem {
                name: "continuation_indents",
                description: "\
Width of continuations, measured as a multiple of the configured indentation.
Continuations are used to further indent the wrapped lines from a \"logical line\".
Indentations are used to indent the base of a \"logical line\".
",
                hint: "<unsigned integer>",
                default: defaults.continuation_indents.to_string(),
            },
            ConfigItem {
                name: "line_ending",
                description: "\
Line ending character sequence.
If \"native\":
  * on Windows, \"crlf\" is used
  * otherwise, \"lf\" is used\
                    ",
                hint: "[ lf | crlf | native ]",
                default: format!("{:?}", defaults.line_ending).to_lowercase(),
            },
        ]
    }
}

pub fn format(config: PasFmtConfiguration<FormattingConfig>, err_handler: impl ErrHandler) {
    let formatting_settings = match config.get_config_object() {
        Ok(formatting_settings) => formatting_settings,
        Err(e) => {
            err_handler(e);
            return;
        }
    };
    log::debug!("Configuration:\n{:#?}", formatting_settings);

    let encoding: &'static Encoding = formatting_settings.encoding.into();
    let formatter = make_formatter(&formatting_settings);
    let file_formatter = FileFormatter::new(formatter, encoding);
    FormattingOrchestrator::run(file_formatter, config, err_handler)
}

pub fn make_formatter(config: &FormattingConfig) -> Formatter {
    let reconstruction_settings: ReconstructionSettings = config.into();

    let eof_newline_formatter = &EofNewline {};

    Formatter::builder()
        .lexer(DelphiLexer {})
        .parser(DelphiLogicalLineParser {})
        .token_consolidator(DistinguishGenericTypeParamsConsolidator {})
        .lines_consolidator(ConditionalDirectiveConsolidator {})
        .token_ignorer(FormattingToggler {})
        .token_ignorer(IgnoreAsmIstructions {})
        .file_formatter(TokenSpacing {})
        .line_formatter(FormatterSelector::new(
            |logical_line_type| match logical_line_type {
                LogicalLineType::Eof => Some(eof_newline_formatter),
                _ => None,
            },
        ))
        .file_formatter(OptimisingLineFormatter::new(
            config.into(),
            reconstruction_settings.clone(),
        ))
        .reconstructor(DelphiLogicalLinesReconstructor::new(
            reconstruction_settings,
        ))
        .build()
}

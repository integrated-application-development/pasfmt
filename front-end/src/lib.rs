#![deny(clippy::enum_glob_use)]

use encoding_rs::Encoding;
use pasfmt_core::prelude::*;
use pasfmt_orchestrator::predule::*;
use serde::Deserialize;

#[cfg(windows)]
fn get_windows_default_encoding() -> &'static Encoding {
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

fn default_encoding() -> &'static Encoding {
    #[cfg(windows)]
    {
        use std::sync::LazyLock;

        static WINDOWS_DEFAULT_ENCODING: LazyLock<&'static Encoding> =
            LazyLock::new(get_windows_default_encoding);

        *WINDOWS_DEFAULT_ENCODING
    }
    #[cfg(not(windows))]
    encoding_rs::UTF_8
}

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum LineEnding {
    #[serde(alias = "CRLF")]
    Crlf,
    #[serde(alias = "LF")]
    Lf,
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

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FormattingConfig {
    #[serde(default = "default_line_ending")]
    line_ending: LineEnding,
    #[serde(default = "default_use_tabs")]
    use_tabs: bool,
    #[serde(default = "default_tab_width")]
    tab_width: u8,
    #[serde(default = "default_continuation_indents")]
    continuation_indents: u8,

    #[serde(default = "Default::default")]
    olf: OlfConfig,

    #[serde(default = "default_encoding")]
    encoding: &'static Encoding,
}

impl FormattingConfig {
    #[cfg(feature = "__demo")]
    pub fn max_line_length(&self) -> u32 {
        self.olf.max_line_length
    }
}

fn default_line_ending() -> LineEnding {
    LineEnding::Native
}

fn default_use_tabs() -> bool {
    false
}

fn default_tab_width() -> u8 {
    2
}

fn default_continuation_indents() -> u8 {
    2
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            line_ending: default_line_ending(),
            use_tabs: default_use_tabs(),
            tab_width: default_tab_width(),
            continuation_indents: default_continuation_indents(),
            encoding: default_encoding(),
            olf: OlfConfig::default(),
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

#[cfg_attr(feature = "__demo", derive(serde::Serialize))]
#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
struct OlfConfig {
    #[serde(default = "default_max_line_length")]
    max_line_length: u32,
}
fn default_max_line_length() -> u32 {
    120
}

impl From<OlfConfig> for OptimisingLineFormatterSettings {
    fn from(value: OlfConfig) -> Self {
        Self {
            max_line_length: value.max_line_length,
            iteration_max: 20_000,
        }
    }
}
impl Default for OlfConfig {
    fn default() -> Self {
        OlfConfig {
            max_line_length: default_max_line_length(),
        }
    }
}

pub fn format(config: PasFmtConfiguration, err_handler: impl ErrHandler) {
    let formatting_settings = match config.get_config_object::<FormattingConfig>() {
        Ok(formatting_settings) => formatting_settings,
        Err(e) => {
            err_handler(e);
            return;
        }
    };
    log::debug!("Configuration:\n{:#?}", formatting_settings);

    let encoding = formatting_settings.encoding;
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
        .token_ignorer(FormattingToggler {})
        .token_ignorer(IgnoreNonUnitImportClauses {})
        .token_ignorer(IgnoreAsmIstructions {})
        .file_formatter(TokenSpacing {})
        .line_formatter(FormatterSelector::new(
            |logical_line_type| match logical_line_type {
                LogicalLineType::Eof => Some(eof_newline_formatter),
                _ => None,
            },
        ))
        .file_formatter(OptimisingLineFormatter::new(
            config.olf.clone().into(),
            reconstruction_settings.clone(),
        ))
        .reconstructor(DelphiLogicalLinesReconstructor::new(
            reconstruction_settings,
        ))
        .build()
}

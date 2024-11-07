use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

use log::error;
use pasfmt::format_with_settings;
use pasfmt::FormattingSettings;
use pasfmt_orchestrator::predule::*;

pasfmt_config!(
    #[command(bin_name = "pasfmt")]
    Config
);

fn main() -> ExitCode {
    let config = Config::create();
    stderrlog::new()
        .verbosity(config.log_level())
        .init()
        .unwrap();

    let had_error = AtomicBool::new(false);

    let err_handler = |e| {
        had_error.store(true, Ordering::Relaxed);
        error!("{:?}", e);
    };

    match config.get_config_object::<FormattingSettings>() {
        Ok(formatting_settings) => {
            format_with_settings(formatting_settings, config, err_handler);
        }
        Err(e) => err_handler(e),
    }

    if had_error.into_inner() {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

use pasfmt::format_with_settings;
use pasfmt::FormattingSettings;
use pasfmt_orchestrator::predule::*;

fn main() {
    let config = PasFmtConfiguration::new();
    stderrlog::new()
        .verbosity(config.log_level())
        .init()
        .unwrap();

    let formatting_settings = config.get_config_object::<FormattingSettings>();
    format_with_settings(formatting_settings, config);
}

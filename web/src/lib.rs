use pasfmt::{make_formatter, FormattingConfig};
use pasfmt_core::prelude::FileOptions;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SettingsWrapper {
    config: FormattingConfig,
}

#[wasm_bindgen]
impl SettingsWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(settings: String) -> Result<Self, String> {
        let config = toml::from_str(&settings).map_err(|e| e.to_string())?;
        Ok(SettingsWrapper { config })
    }

    #[wasm_bindgen]
    pub fn max_line_len(&self) -> u32 {
        self.config.max_line_length()
    }
}

#[wasm_bindgen]
pub fn default_settings_toml() -> String {
    toml::to_string_pretty(&FormattingConfig::default()).unwrap()
}

#[wasm_bindgen]
pub fn fmt(src: &str, settings: &SettingsWrapper) -> Result<String, String> {
    let formatter = make_formatter(&settings.config).map_err(|e| e.to_string())?;

    Ok(formatter.format(src, FileOptions::new()))

}

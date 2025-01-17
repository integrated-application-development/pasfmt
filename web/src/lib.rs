use pasfmt::{make_formatter, FormattingSettings};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct SettingsWrapper {
    formatting_settings: FormattingSettings,
}

#[wasm_bindgen]
impl SettingsWrapper {
    #[wasm_bindgen(constructor)]
    pub fn new(settings: String) -> Result<Self, String> {
        let formatting_settings = toml::from_str(&settings).map_err(|e| e.to_string())?;
        Ok(SettingsWrapper {
            formatting_settings,
        })
    }

    #[wasm_bindgen]
    pub fn max_line_len(&self) -> u32 {
        self.formatting_settings.max_line_length()
    }
}

#[wasm_bindgen]
pub fn default_settings_toml() -> String {
    toml::to_string_pretty(&FormattingSettings::default()).unwrap()
}

#[wasm_bindgen]
pub fn fmt(src: &str, settings: &SettingsWrapper) -> Result<String, String> {
    let formatter = make_formatter(&settings.formatting_settings).map_err(|e| e.to_string())?;

    Ok(formatter.format(src))
}

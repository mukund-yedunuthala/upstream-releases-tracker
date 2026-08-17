use crate::Config;
use crate::json_handler;
use system_theme::{SystemTheme, ThemeScheme};

#[derive(serde::Serialize)]
pub struct ThemeSettings {
    pub mode: String,
    pub system_scheme: String,
    pub accent: Option<AccentColor>,
}

#[derive(serde::Serialize)]
pub struct AccentColor {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

pub fn query_system_theme(config: &Config) -> ThemeSettings {
    let theme = SystemTheme::new().ok();

    let system_scheme = theme
        .as_ref()
        .and_then(|t| t.get_scheme().ok())
        .map(|s| match s {
            ThemeScheme::Light => "light",
            ThemeScheme::Dark => "dark",
        })
        .unwrap_or_else(|| "light");

    let accent = theme
        .as_ref()
        .and_then(|t| t.get_accent().ok())
        .map(|c| AccentColor {
            red: c.red as f64,
            green: c.green as f64,
            blue: c.blue as f64,
        });

    ThemeSettings {
        mode: config.theme_mode.clone(),
        system_scheme: system_scheme.to_string(),
        accent,
    }
}

pub fn persist_theme_mode(config: &mut Config, mode: &str) -> Result<(), String> {
    match mode {
        "system" | "light" | "dark" => {}
        _ => return Err(format!("Invalid theme mode: {}", mode)),
    }
    config.theme_mode = mode.to_string();
    let config_path = crate::config_handler::config_path_string()
        .map_err(|e| format!("Failed to determine config path: {}", e))?;
    let persisted_config = crate::scrub_keys(config);
    json_handler::write_json_file::<Config>(&config_path, &persisted_config)
        .map_err(|e| format!("Failed to persist theme mode: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn persist_theme_mode_valid_modes() {
        let mut config = Config::default();
        for mode in ["system", "light", "dark"] {
            config.theme_mode = "system".to_string();
            let result = persist_theme_mode(&mut config, mode);
            assert!(result.is_ok(), "persist_theme_mode({}) failed", mode);
            assert_eq!(config.theme_mode, mode);
        }
    }

    #[test]
    fn persist_theme_mode_invalid_mode() {
        let mut config = Config::default();
        let result = persist_theme_mode(&mut config, "invalid");
        assert!(result.is_err());
    }

    #[test]
    fn default_theme_mode_is_system() {
        let config = Config::default();
        assert_eq!(config.theme_mode, "system");
    }
}

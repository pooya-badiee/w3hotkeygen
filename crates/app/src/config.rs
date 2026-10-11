//! Tiny `key=value` config in `%APPDATA%\w3hotkeygen`.

use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct Config {
    pub wc3_path: Option<PathBuf>,
}

fn config_file() -> Option<PathBuf> {
    let app_data = PathBuf::from(std::env::var_os("APPDATA")?);
    Some(app_data.join("w3hotkeygen").join("config.ini"))
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = Config::default();
        let Some(text) = config_file().and_then(|f| fs::read_to_string(f).ok()) else { return cfg };
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=')
                && key.trim() == "wc3_path" && !value.trim().is_empty() {
                    cfg.wc3_path = Some(PathBuf::from(value.trim()));
                }
        }
        cfg
    }

    pub fn save(&self) -> std::io::Result<()> {
        let file = config_file().ok_or_else(|| std::io::Error::other("no config directory"))?;
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir)?;
        }
        let path = self.wc3_path.as_ref().map(|p| p.display().to_string()).unwrap_or_default();
        fs::write(file, format!("wc3_path={path}\n"))
    }
}

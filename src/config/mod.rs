use std::{fs, io::Read};

use serde::Deserialize;

#[derive(Deserialize, Default)]
pub struct Config {
    pub(crate) headers: Option<Vec<(String, String)>>,
    pub(crate) concurrency: Option<u64>,
    pub(crate) chunk_size: Option<String>,
    pub(crate) timeout: Option<u64>,
    pub(crate) dns_timeout: Option<u64>,
    pub(crate) retries: Option<u64>,
    pub(crate) retry_wait: Option<u64>,
}

impl Config {
    pub fn new() -> Config {
        if let Some(path) = dirs::home_dir() {
            let config_dir = path.join(".config").join("aget");
            if config_dir.is_dir() {
                let config_path = config_dir.join("config");
                if config_path.exists() && config_path.is_file() {
                    let mut fl = fs::File::open(&config_path)
                        .expect(&format!("Can't open configuration file: {:?}", config_path));
                    let mut cn = String::new();
                    fl.read_to_string(&mut cn).unwrap();
                    let config: Config = toml::from_str(&cn).unwrap();
                    return config;
                }
            }
        }
        Config::default()
    }
}

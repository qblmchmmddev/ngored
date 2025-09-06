use std::{
    fs::{self, create_dir_all},
    path::PathBuf,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub subs: Vec<String>,
}

impl Config {
    pub fn new(subs: Vec<String>) -> Self {
        Self { subs }
    }
    pub fn load() -> Self {
        let path = Self::path();
        let data = fs::read_to_string(path);
        if let Ok(data) = data {
            toml::from_str(&data).expect("Invalid config file")
        } else {
            Self {
                subs: Vec::default(),
            }
        }
    }
    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            create_dir_all(parent).expect("Cannot create config directory");
        }
        let data = toml::to_string_pretty(self).expect("Cannot save config");
        fs::write(path, data).expect("Cannot save config");
    }

    fn path() -> PathBuf {
        let home = dirs::home_dir().expect("Could not find home directory");
        home.join(".config").join("ngored").join("config.toml")
    }
}

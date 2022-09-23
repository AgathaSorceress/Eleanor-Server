use std::{fs::File, io::Write};

use miette::{IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Source {
    pub id: u8,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub port: u16,
    pub sources: Vec<Source>,
}
impl Config {
    pub fn read_config() -> Result<Self> {
        let file = std::env::current_dir()
            .map(|v| v.join("settings.toml"))
            .into_diagnostic()?;
        let contents = std::fs::read_to_string(file).into_diagnostic()?;

        toml::from_str(&contents).into_diagnostic()
    }

    pub fn write_config(config: &Config) -> Result<()> {
        let contents = toml::to_string(config).into_diagnostic()?;

        let path = std::env::current_dir()
            .map(|v| v.join("settings.toml"))
            .into_diagnostic()?;

        File::create(path)
            .and_then(|mut v| v.write_all(contents.as_bytes()))
            .into_diagnostic()
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            port: 8008,
            sources: vec![Source {
                id: 0,
                path: "".into(),
            }],
        }
    }
}

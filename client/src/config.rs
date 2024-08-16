use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct UserConfig {
    pub name: String,
    pub id: String,
}

impl From<&str> for UserConfig {
    fn from(name: &str) -> Self {
        UserConfig {
            name: name.to_string(),
            id: Uuid::new_v4().to_string(),
        }
    }
}

fn get_config_path() -> Result<PathBuf> {
    let app_data_dir = match dirs::data_dir() {
        Some(dir) => dir,
        None => {
            bail!("No %APPDATA% directory found.")
        }
    };

    let app_dir = app_data_dir.join("nivalis");

    std::fs::create_dir_all(&app_dir).context("Couldn't create %APPDATA%/nivalis directory.")?;

    Ok(app_dir.join("cli.json"))
}

pub fn save_config(username: &str) -> Result<()> {
    let config_path = get_config_path()?;
    let file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(config_path)
        .context("Couldn't read or create your config file.")?;

    let mut writer = BufWriter::new(file);

    serde_json::to_writer(&mut writer, &UserConfig::from(username))?;

    writer.flush().context("Couldn't save your config.")
}

pub async fn get_config() -> Result<UserConfig> {
    let config_path = get_config_path()?;
    let file = File::open(config_path).context("Couldn't read your config file.")?;
    let reader = BufReader::new(file);
    let config = serde_json::from_reader(reader)?;

    Ok(config)
}

pub fn delete_config() -> Result<()> {
    let auth_config_path = get_config_path()?;
    std::fs::remove_file(auth_config_path).context("Couldn't delete your config.")
}

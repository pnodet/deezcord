use anyhow::Result;
use std::fs;
use std::io::{self, Write};

use crate::config::{create_config, get_config, get_config_path, UserConfig};

pub fn ask_for_username() -> String {
    let mut input = String::new();
    print!("Enter your name: ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}

pub fn get_user_or_create() -> Result<UserConfig> {
    let user = match get_config() {
        Ok(user) => user,
        Err(_) => {
            let name = ask_for_username();
            let user = create_config(&name)?;
            user
        }
    };

    Ok(user)
}

pub fn set_user(user: &UserConfig) {
    let path = get_config_path();
    if let Ok(data) = serde_json::to_string(user) {
        if let Ok(path) = path {
            fs::write(path, data).expect("Unable to write user data");
        }
    }
}

pub fn set_id(id: &str) {
    if let Ok(mut user) = get_user_or_create() {
        user.id = id.to_string();
        set_user(&user);
    }
}

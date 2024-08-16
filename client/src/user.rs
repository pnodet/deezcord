use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use uuid::Uuid;

const USER_DATA_FILE: &str = "user_data.json";

#[derive(Serialize, Deserialize)]
pub struct ClientUser {
    pub name: String,
    pub id: String,
}

impl ClientUser {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            id: Uuid::new_v4().to_string(),
        }
    }
}

pub fn get_user() -> Option<ClientUser> {
    if let Ok(data) = fs::read_to_string(USER_DATA_FILE) {
        if let Ok(user) = serde_json::from_str::<ClientUser>(&data) {
            return Some(user);
        }
    }
    None
}

pub fn set_user(user: &ClientUser) {
    if let Ok(data) = serde_json::to_string(user) {
        fs::write(USER_DATA_FILE, data).expect("Unable to write user data");
    }
}

pub fn set_id(id: &str) {
    if let Some(mut user) = get_user() {
        user.id = id.to_string();
        set_user(&user);
    }
}

pub fn ask_for_username() -> String {
    let mut input = String::new();
    print!("Enter your name: ");
    io::stdout().flush().unwrap();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    input.trim().to_string()
}

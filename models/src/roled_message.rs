use std::fmt::Display;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Ai,
    User,
}

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Role::Ai => "Ai",
            Role::User => "You",
        };

        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoledMessage {
    pub message: String,
    pub role: Role,
}

impl Display for RoledMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

#[derive(Deserialize)]
pub struct AiResponse {
    #[serde(rename = "action")]
    pub message: String,
}

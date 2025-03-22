use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::collections::HashMap;

use crate::task::model::TaskStatus;

/// Represents a Task for data transfer over the API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskResponse {
    pub id: Uuid,
    pub name: String,
    pub status: TaskStatus,
}

/// Represents Alias configuration for data transfer over the API.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AliasResponse {
    pub alias: String,
    pub intent: String,
    pub parameters: Option<HashMap<String, String>>,
    pub command_type: Option<String>,
    pub steps: Option<Vec<AliasResponse>>,
}

/// Structures for Settings.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SettingsResponse {
    pub aliases: Vec<AliasResponse>,
    pub language: String,
    pub notification_enable: bool,
    pub antiflood: bool,
    pub notifications_delay: u32,
    // Add all AppConfig fields here
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub value: String,
}

/// Represents a command execution request.
#[derive(Debug, Deserialize)]
pub struct ExecuteCommandRequest {
    pub query: String,
}

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::fmt;

/// Represents the status of a task.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Failed(String), // Include the error message if the task failed
    Cancelled,
    Stopping
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskStatus::Queued => write!(f, "Queued"),
            TaskStatus::Running => write!(f, "Running"),
            TaskStatus::Completed => write!(f, "Completed"),
            TaskStatus::Failed(e) => write!(f, "Failed: {}", e),
            TaskStatus::Cancelled => write!(f, "Cancelled"),
            TaskStatus::Stopping => write!(f, "Stopping")
        }
    }
}

/// Represents a task in the system.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TaskInfo {
    pub id: Uuid,           // Unique identifier for the task
    pub name: String,         // Name or description of the task
    pub status: TaskStatus, // Current status of the task
    // Add more fields as needed (e.g., start time, end time, etc.)
}

impl TaskInfo {
    pub fn new(name: String) -> Self {
        TaskInfo {
            id: Uuid::new_v4(), // Generate a new UUID
            name,
            status: TaskStatus::Queued,
        }
    }
}

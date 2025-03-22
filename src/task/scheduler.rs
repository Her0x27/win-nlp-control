use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

use crate::config::SharedConfig;
use crate::platform::windows::controller::WinUiController;
use crate::task::model::{TaskInfo, TaskStatus};
use log::{info, error};

/// A task that can be scheduled by the TaskScheduler.
/// Each task has a name for identification and a closure representing the action to execute.
pub struct Task {
    pub id: Uuid,
    pub name: String,
    pub action: Box<dyn FnOnce() -> Result<(), String> + Send + 'static>,
}

impl Task {
    /// Creates a new task with the given name and action.
    pub fn new<F>(name: &str, action: F) -> Self
    where
        F: FnOnce() -> Result<(), String> + Send + 'static,
    {
        Task {
            id: Uuid::new_v4(),
            name: name.to_string(),
            action: Box::new(action),
        }
    }
}

/// TaskScheduler manages a queue of tasks and executes them sequentially on a background thread.
pub struct TaskScheduler {
    sender: mpsc::Sender<Task>,
    shared_config: SharedConfig,
    controller: Arc<WinUiController>,  // Add reference to WinUiController
}

impl TaskScheduler {
    /// Creates a new TaskScheduler and starts a worker thread that processes tasks.
    /// The scheduler uses the shared configuration to display notifications based on language messages and settings.
    pub fn new(shared_config: SharedConfig, controller: Arc<WinUiController>) -> Self {
        let (tx, rx) = mpsc::channel::<Task>();

        let shared_config_clone = shared_config.clone();
        let controller_clone = controller.clone(); // Clone the WinUiController

        // Spawn a worker thread that processes tasks.
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(task) => {
                        let task_id = task.id;
                        let task_name = task.name.clone();
                        info!("Task ID {} Recieved", task_id);
                         let config_lock = shared_config_clone.lock().unwrap();
                            if let Some(ref cfg) = *config_lock {
                                info!("Task ID {}: {}", task_id, task_name);
                            }
                        // Execute the task.
                        let result = (task.action)();

                         let config_lock = shared_config_clone.lock().unwrap();
                            if let Some(ref cfg) = *config_lock {
                                match result {
                                     Ok(_) => {
                                        info!("Task ID {}: Completed", task_id);
                                     },
                                     Err(e) => {
                                         error!("Task ID {} Failed, error - {}", task_id, e)
                                     }
                                }
                            }

                    }
                    Err(_) => {
                        // If the channel is disconnected, exit the worker loop.
                        break;
                    }
                }
            }
        });

        TaskScheduler { sender: tx, shared_config, controller: controller_clone }
    }

    /// Schedules a new task for execution.
    /// If sending the task fails, an error is logged to the console.
    pub fn schedule(&self, task: Task) {
        if let Err(e) = self.sender.send(task) {
            error!("Error scheduling task: {}", e);
        }
    }
}

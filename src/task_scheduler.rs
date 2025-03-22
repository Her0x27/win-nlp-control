use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::config::{AppConfig, SharedConfig};
use crate::language::PATTERNS;

/// A task that can be scheduled by the TaskScheduler.
/// Each task has a name for identification and a closure representing the action to execute.
pub struct Task {
    pub name: String,
    pub action: Box<dyn FnOnce() + Send + 'static>,
}

impl Task {
    /// Creates a new task with the given name and action.
    pub fn new<F>(name: &str, action: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        Task {
            name: name.to_string(),
            action: Box::new(action),
        }
    }
}

/// TaskScheduler manages a queue of tasks and executes them sequentially on a background thread.
pub struct TaskScheduler {
    sender: mpsc::Sender<Task>,
}

impl TaskScheduler {
    /// Creates a new TaskScheduler and starts a worker thread that processes tasks.
    /// The scheduler uses the shared configuration to display notifications based on language messages and settings.
    pub fn new(shared_config: SharedConfig) -> Self {
        let (tx, rx) = mpsc::channel::<Task>();

        // Spawn a worker thread that processes tasks.
        thread::spawn(move || {
            loop {
                match rx.recv() {
                    Ok(task) => {
                        // Load current configuration to display notifications.
                        if let Ok(config_lock) = shared_config.lock() {
                            if let Some(ref cfg) = *config_lock {
                                // Notify that the task has been queued.
                                cfg.show_notification(&format!(
                                    "{}: {}",
                                    PATTERNS.msg_task_queued, task.name
                                ));
                                
                                // Wait for the configured notification delay.
                                thread::sleep(Duration::from_millis(cfg.notifications_delay as u64));
                                
                                // Notify that the task is now processing.
                                cfg.show_notification(&format!(
                                    "{}: {}",
                                    PATTERNS.msg_task_processing, task.name
                                ));
                            }
                        }
                        
                        // Execute the task.
                        (task.action)();
                        
                        // After executing, notify that the task was successfully completed.
                        if let Ok(config_lock) = shared_config.lock() {
                            if let Some(ref cfg) = *config_lock {
                                cfg.show_notification(&format!(
                                    "{}: {}",
                                    PATTERNS.msg_task_success, task.name
                                ));
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

        TaskScheduler { sender: tx }
    }

    /// Schedules a new task for execution.
    /// If sending the task fails, an error is logged to the console.
    pub fn schedule(&self, task: Task) {
        if let Err(e) = self.sender.send(task) {
            eprintln!("Error scheduling task: {}", e);
        }
    }
}
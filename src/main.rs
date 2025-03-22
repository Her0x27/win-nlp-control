use actix_web::{get, put, App, HttpResponse, HttpServer, Responder, web, Result};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tokio::sync::oneshot; // For task cancellation
use tokio::task::JoinHandle;
use uuid::Uuid; // For generating unique task IDs
use std::time::Duration;
use actix_web::http::header::ContentType;
use std::fs;
use log::{info, error, debug}; // Import logging macros
use env_logger::Env;

// Добавьте ваши модули:
mod config;
mod language;
mod intent_mapper;
mod nlp;
mod task_scheduler;
mod winui_controller;
mod debug_logger;

use crate::config::{AppConfig, SharedConfig, init_shared_config};
use crate::nlp::parse_command;
use crate::intent_mapper::map_intent;
use crate::winui_controller::execute_action;
use crate::task_scheduler::{Task, TaskScheduler};
use crate::language::PATTERNS; // Import PATTERNS

// Task structure (replace with your actual Task structure)
#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskInfo {
    id: Uuid, // Уникальный идентификатор задачи
    name: String,
    status: String, // e.g., "queued", "running", "completed", "error"
    // Optional: Add more fields to describe the task
}

// State to hold tasks
struct AppState {
    tasks: Arc<Mutex<HashMap<Uuid, (TaskInfo, Option<oneshot::Sender<()>>, Option<JoinHandle<()>>> >>,
    config: SharedConfig,  // Shared configuration
    scheduler: Arc<TaskScheduler>,   // Your TaskScheduler
    config_path: String, // Store the config file path
}

// 1. Handler for command processing
#[get("/")]
async fn execute_command(data: web::Data<AppState>, query: web::Query<HashMap<String, String>>) -> impl Responder {
    let command = query.get("query").cloned().unwrap_or_else(|| "help".to_string());
    info!("Received command: {}", command);

    let nlp_result = parse_command(&command);
    debug!("NLP Result: {:?}", nlp_result);

    let action = map_intent(&nlp_result, &data.config);
    debug!("Mapped Action: {:?}", action);

    let task_name = format!("Task: {}", command);
    let task_id = Uuid::new_v4(); // Generate a unique task ID

    // Create a channel for task cancellation
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

    let task_action = {
        let config = data.config.clone();
        let task_id = task_id.clone(); // Capture the task ID
        let tasks_clone = data.tasks.clone(); // Capture the task list
        move || {
             info!("Executing task: {}", task_name);
            let action_result = execute_action(&action);

            // Log or handle action_result within the task if needed
             info!("Task completed with result: {:?}", action_result);

            // Update the task status
            let mut tasks_lock = tasks_clone.lock().unwrap();
            if let Some((task_info, _, _)) = tasks_lock.get_mut(&task_id) {
                task_info.status = format!("{:?}", action_result); // Update with actual result
            }
        }
    };

    let task = Task::new(&task_name, task_action);

    // Create TaskInfo
    let task_info = TaskInfo {
        id: task_id,
        name: task_name.clone(),
        status: "queued".to_string(), // Initial status
    };

    // Add task to the list
    {
        let mut tasks_lock = data.tasks.lock().unwrap();
        tasks_lock.insert(task_id, (task_info.clone(), Some(cancel_tx), None));
    }

    // Spawn the task using Tokio
    let scheduler_clone = data.scheduler.clone(); // Clone the scheduler
    let task_id_clone = task_id.clone(); // Clone the task ID for the spawned task
    let tasks_clone_2 = data.tasks.clone(); // Clone task
    let handle: JoinHandle<()> = tokio::spawn(async move {
            // Schedule task
            scheduler_clone.schedule(task);

            // Await for cancellation
            tokio::select! {
                _ = cancel_rx => {
                    info!("Task {} cancelled.", task_id_clone);
                      let mut tasks_lock = tasks_clone_2.lock().unwrap();
                    if let Some((task_info, _, _)) = tasks_lock.get_mut(&task_id_clone) {
                        task_info.status = "cancelled".to_string(); // Update with actual result
                    }
                }
            }
           
        });

     // Update task list with JoinHandle
        {
            let mut tasks_lock = data.tasks.lock().unwrap();
            if let Some((task_info, _, _)) = tasks_lock.get_mut(&task_id) {
                tasks_lock.insert(task_id, (task_info.clone(), Some(cancel_tx), Some(handle)));
            }
        }

     HttpResponse::Ok().content_type(ContentType::plaintext()).body(format!("Task '{}' scheduled with id {}.", command, task_id))
}

// 2. Handler to get the task list
#[get("/get=tasksall")]
async fn get_all_tasks(data: web::Data<AppState>) -> impl Responder {
    let tasks_lock = data.tasks.lock().unwrap();
    let task_list: Vec<TaskInfo> = tasks_lock.iter().map(|(_, (task_info, _, _))| task_info.clone()).collect();
    HttpResponse::Ok().json(task_list)
}

// 3. Handler to stop a task
#[get("/stop={task_id}")]
async fn stop_task(data: web::Data<AppState>, task_id: web::Path<Uuid>) -> impl Responder {
    let id = task_id.into_inner();
    info!("Stopping task with id: {}", id);

    let mut tasks_lock = data.tasks.lock().unwrap();

    if let Some((task_info, cancel_tx_opt, join_handle_opt)) = tasks_lock.remove(&id) {
        task_info.status = "stopping".to_string(); // Set status to "stopping"

        if let Some(cancel_tx) = cancel_tx_opt {
            let _ = cancel_tx.send(()); // Signal cancellation
            info!("Sent cancellation signal for task {}", id);
        }

        if let Some(join_handle) = join_handle_opt {
             info!("Aborting tokio handle for task {}", id);
            join_handle.abort(); // Abort the spawned task
        }
         HttpResponse::Ok().content_type(ContentType::plaintext()).body(format!("Stopping task with id: {}", id))
    } else {
        HttpResponse::NotFound().body(format!("Task with id {} not found", id))
    }
}

// 4. Handler to get the status
#[get("/status")]
async fn get_status() -> impl Responder {
    HttpResponse::Ok().content_type(ContentType::plaintext()).body("Status: Running")
}

// 5. Handler to get settings
#[get("/get=settings")]
async fn get_settings(data: web::Data<AppState>) -> impl Responder {
    let config_lock = data.config.lock().unwrap();
    if let Some(ref cfg) = *config_lock {
        HttpResponse::Ok().json(&cfg)
    } else {
        HttpResponse::NotFound().body("Settings not initialized")
    }
}

// 6. Handler to get settings by name
#[get("/get=settings.{setting_name}")]
async fn get_setting_by_name(data: web::Data<AppState>, setting_name: web::Path<String>) -> impl Responder {
    let name = setting_name.into_inner();
    let config_lock = data.config.lock().unwrap();
    if let Some(ref cfg) = *config_lock {
        match name.as_str() {
            "notifications_delay" => HttpResponse::Ok().content_type(ContentType::plaintext()).body(cfg.notifications_delay.to_string()),
            "language" => HttpResponse::Ok().content_type(ContentType::plaintext()).body(cfg.language.clone()),
            _ => HttpResponse::NotFound().body("Setting not found"),
        }
    } else {
        HttpResponse::NotFound().body("Settings not initialized")
    }
}

// 7. Handler to update settings
#[put("/put=settings.{setting_name}")]
async fn update_setting(data: web::Data<AppState>, path: web::Path<String>, query: web::Query<HashMap<String, String>>) -> impl Responder {
    let setting_path = path.into_inner();
    let app_state = data.clone();
    if let Some((config_lock, mut json_result)) = update_config(&data.config, &data.config_path, &setting_path, query).await {
       
        if json_result.is_ok() {
             HttpResponse::Ok().content_type(ContentType::plaintext()).body(format!("{}", json_result.unwrap()))
        } else {
              HttpResponse::BadRequest().body(json_result.unwrap_err().to_string())
        }
    } else {
         HttpResponse::NotFound().body("Settings not initialized")
    }
}

//Helper to perform safe config update
async fn update_config(config: &SharedConfig, config_path: &str, setting_path: &str, query: web::Query<HashMap<String, String>>) -> Option<(SharedConfig,  Result<String, Box<dyn std::error::Error>>>) {
     let mut config_lock = config.lock().unwrap();
    if let Some(ref mut cfg) = *config_lock {
        if let Some(value) = query.get("value") {
            let result: Result<String, Box<dyn std::error::Error>> = match setting_path {
                "notifications_delay" => {
                     match value.parse::<u32>() {
                         Ok(new_delay) => {
                              cfg.notifications_delay = new_delay;
                               Ok(format!("Notification delay updated to {}", new_delay))
                         },
                         Err(e) => {
                              Err(From::from("value is not in the right type, please try again"))
                         }
                     }
                },
                "language" => {
                    cfg.language = value.clone();
                    Ok(format!("Language updated to {}", value))
                },
                _ =>  Err(From::from("Setting not found"))
            };

           if result.is_ok() {
                 let save_result = save_config_to_file(config.clone(), config_path);
                  if save_result.is_err() {
                       error!("Failed to save config to file: {}", save_result.err().unwrap());
                  }
           }

           Some((config.clone(), result))
        } else {
             Some((config.clone(), Err(From::from("Missing 'value' parameter"))))
        }
    } else {
        None
    }
}

// Helper function to save the configuration to a file
fn save_config_to_file(config: SharedConfig, file_path: &str) -> Result<(), String> {
    let config_lock = config.lock().unwrap();
    if let Some(ref cfg) = *config_lock {
        let json_str = serde_json::to_string_pretty(&cfg)
            .map_err(|e| format!("Failed to serialize config to JSON: {}", e))?;
        fs::write(file_path, json_str)
            .map_err(|e| format!("Failed to write config to file: {}", e))?;
        Ok(())
    } else {
        Err("Settings not initialized".to_string())
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging
    env_logger::init_from_env(Env::default().default_filter_or("info"));

     // Initialize configuration
    let config_path = "natural.config"; // Путь к вашему файлу конфигурации
    let shared_config: SharedConfig = init_shared_config(config_path);
    let scheduler = Arc::new(TaskScheduler::new(shared_config.clone()));

    // Example task list (replace with your actual task management)
    let tasks = Arc::new(Mutex::new(HashMap::new())); // Use a HashMap for task management

    let app_state = web::Data::new(AppState {
        tasks: tasks.clone(),
        config: shared_config.clone(),
        scheduler: scheduler.clone(),
        config_path: config_path.to_string(),
    });

    HttpServer::new(move || {
        App::new()
            .app_data(app_state.clone()) // Pass the shared state
            .service(execute_command)
            .service(get_all_tasks)
            .service(stop_task)
            .service(get_status)
            .service(get_settings)
            .service(get_setting_by_name)
            .service(update_setting)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}

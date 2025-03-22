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
use crate::task::model::TaskStatus;

// Добавьте ваши модули:
mod config;
mod language;
mod intent_mapper;
mod nlp;
mod task_scheduler;
mod winui_controller;
mod debug_logger;

mod platform;

use crate::config::{AppConfig, SharedConfig, init_shared_config};
use crate::nlp::parse_command;
use crate::intent_mapper::map_intent;
use crate::task_scheduler::{Task, TaskScheduler};
use crate::language::PATTERNS; // Import PATTERNS
use crate::webapi::models::*;

use crate::platform::windows::controller::WinUiController;

use std::time::Instant;

lazy_static::lazy_static! {
    static ref LAST_COMMAND_TIME: Mutex<Option<Instant>> = Mutex::new(None);
}

// Task structure (replace with your actual Task structure)
#[derive(Debug, Serialize, Deserialize, Clone)]
struct TaskInfo {
    id: Uuid, // Уникальный идентификатор задачи
    name: String,
    status: TaskStatus, // e.g., "queued", "running", "completed", "error"
    // Optional: Add more fields to describe the task
}

// State to hold tasks
struct AppState {
    tasks: Arc<Mutex<HashMap<Uuid, (TaskInfo, Option<oneshot::Sender<()>>, Option<JoinHandle<()>>> >>,
    config: SharedConfig,  // Shared configuration
    scheduler: Arc<TaskScheduler>,   // Your TaskScheduler
    controller: Arc<WinUiController>,
    config_path: String, // Store the config file path
}

#[derive(Serialize)]
struct ErrorResponse {
    message: String,
}

// 1. Handler for command processing
#[get("/")]
async fn execute_command(data: web::Data<AppState>, query: web::Query<ExecuteCommandRequest>) -> HttpResponse {
    let command = &query.query;
    info!("Received command: {}", command);

     let config_lock = data.config.lock().unwrap();
     let (antiflood, antiflood_delay) = if let Some(ref cfg) = *config_lock {
        (cfg.antiflood, cfg.notifications_delay)
    } else {
        (false, 5) // Default values if config is not loaded
    };

    if antiflood {
        let mut last_command_time = LAST_COMMAND_TIME.lock().unwrap();
            let now = Instant::now();
        if let Some(last_time) = *last_command_time {
            let elapsed = now.duration_since(last_time);
              let duration = Duration::from_secs(antiflood_delay as u64);
             if elapsed < duration {
                    let message = format!("Too many requests. Please wait before sending another command. Timeout = {:.2?}", duration - elapsed);
                 let error_response = ErrorResponse { message };
                return HttpResponse::TooManyRequests().json(&error_response);
            }
        }
        *last_command_time = Some(now);
    }

    let nlp_result = parse_command(&command);
    debug!("NLP Result: {:?}", nlp_result);

    let action = map_intent(&nlp_result, &data.config);
    debug!("Mapped Action: {:?}", action);

    let task_name = format!("Task: {}", command);
    let task_id = Uuid::new_v4();

    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    let controller = data.controller.clone();  // Clone the WinUiController
    let task_action = {
        let config = data.config.clone();
        let task_id = task_id.clone();
        let tasks_clone = data.tasks.clone();
        let controller_clone = controller.clone();
        let action_clone = action.clone();
        move || {
            info!("Executing task: {}", task_name);

                let action_result = crate::task::executor::execute_action_on_platform(&action_clone, &controller_clone);

            info!("Task completed with result: {:?}", action_result);

             let mut tasks_lock = tasks_clone.lock().unwrap();
            if let Some((task_info, _, _)) = tasks_lock.get_mut(&task_id) {
                task_info.status = match action_result {
                    Ok(_) => TaskStatus::Completed,
                    Err(e) => TaskStatus::Failed(e),
                };
            }
        }
    };

    let task = Task::new(&task_name, task_action);

    let task_info = TaskInfo {
        id: task_id,
        name: task_name.clone(),
        status: TaskStatus::Queued,
    };

    {
        let mut tasks_lock = data.tasks.lock().unwrap();
        tasks_lock.insert(task_id, (task_info.clone(), Some(cancel_tx), None));
    }

    let scheduler_clone = data.scheduler.clone();
    let task_id_clone = task_id.clone();
    let tasks_clone_2 = data.tasks.clone();
    let handle: JoinHandle<()> = tokio::spawn(async move {
        // Schedule task
        scheduler_clone.schedule(task);

        // Await for cancellation
        tokio::select! {
            _ = cancel_rx => {
                info!("Task {} cancelled.", task_id_clone);
                let mut tasks_lock = tasks_clone_2.lock().unwrap();
                if let Some((task_info, _, _)) = tasks_lock.get_mut(&task_id_clone) {
                    task_info.status = TaskStatus::Cancelled;
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
    HttpResponse::Ok().json(&task_info) // Return TaskInfo
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
        //task_info.status = "stopping".to_string(); // Set status to "stopping"

        if let Some(cancel_tx) = cancel_tx_opt {
            let _ = cancel_tx.send(()); // Signal cancellation
            info!("Sent cancellation signal for task {}", id);
        }

        if let Some(join_handle) = join_handle_opt {
             info!("Aborting tokio handle for task {}", id);
            join_handle.abort(); // Abort the spawned task
        }
       HttpResponse::Ok().json(task_info)
    } else {
          let message = format!("Task with id {} not found", id);
        let error_response = ErrorResponse { message };
        HttpResponse::NotFound().json(&error_response)
    }
}

// 4. Handler to get the status
#[get("/status")]
async fn get_status() -> impl Responder {
     let message = "Status: Running".to_string();
        let response = ErrorResponse { message };
    HttpResponse::Ok().json(response)
}

// 5. Handler to get settings
#[get("/get=settings")]
async fn get_settings(data: web::Data<AppState>) -> impl Responder {
    let config_lock = data.config.lock().unwrap();
    if let Some(ref cfg) = *config_lock {
         let settings_response = SettingsResponse {
                aliases: cfg.aliases.clone(),
                language: cfg.language.clone(),
                notification_enable: cfg.notification_enable,
                antiflood: cfg.antiflood,
                notifications_delay: cfg.notifications_delay
          };
        HttpResponse::Ok().json(&settings_response)
    } else {
          let message = "Settings not initialized".to_string();
        let error_response = ErrorResponse { message };
        HttpResponse::NotFound().json(error_response)
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
            "notification_enable" =>  HttpResponse::Ok().json(&cfg.notification_enable),
            "antiflood" => HttpResponse::Ok().json(&cfg.antiflood),
            _ =>  {
                  let message = "Settings not found".to_string();
                  let error_response = ErrorResponse { message };
                  HttpResponse::NotFound().json(&error_response)
            } ,
        }
    } else {
          let message = "Settings not initialized".to_string();
                  let error_response = ErrorResponse { message };
                HttpResponse::NotFound().json(error_response)
    }
}

// 7. Handler to update settings
#[put("/put=settings.{setting_name}")]
async fn update_setting(data: web::Data<AppState>, path: web::Path<String>, query: web::Query<HashMap<String, String>>) -> impl Responder {
    let setting_path = path.into_inner();
    let app_state = data.clone();
    if let Some((config_lock, mut json_result)) = update_config(&data.config, &data.config_path, &setting_path, query).await {
           if json_result.is_ok() {
            let message = json_result.unwrap();
             let response = ErrorResponse { message };
             HttpResponse::Ok().json(response)
           } else {
                 let message = json_result.unwrap_err().to_string();
                 let response = ErrorResponse { message };
               HttpResponse::BadRequest().json(response)
           }
    } else {
                 let message = "Settings not initialized".to_string();
                 let response = ErrorResponse { message };
        HttpResponse::NotFound().json(response)
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
                 "notification_enable" => {
                    match value.parse::<bool>() {
                        Ok(new_value) => {
                            cfg.notification_enable = new_value;
                            Ok(format!("Notification enable updated to {}", new_value))
                        }
                        Err(_) => Err(From::from("Invalid value for notification_enable. Must be a boolean (true/false)"))
                    }
                }
                "antiflood" => {
                    match value.parse::<bool>() {
                        Ok(new_value) => {
                            cfg.antiflood = new_value;
                            Ok(format!("Anti-flood updated to {}", new_value))
                        }
                        Err(_) => Err(From::from("Invalid value for antiflood. Must be a boolean (true/false)"))
                    }
                }
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

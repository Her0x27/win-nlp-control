mod config;
mod language;
mod intent_mapper;
mod nlp;
mod task_scheduler;
mod winui_controller;
//mod debug_logger;

pub mod prelude {
    pub use crate::config::*;
    pub use crate::language::*;
    pub use crate::intent_mapper::*;
    pub use crate::nlp::*;
    pub use crate::task_scheduler::*;
    pub use crate::winui_controller::*;
    // pub use crate::logger::*;
}

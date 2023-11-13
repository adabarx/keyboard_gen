
pub mod factory;

use crate::factory::Keyboard;

#[derive(Clone)]
pub enum AppState {
    Init,
    Running {
        batch_size: usize,
        completed: usize,
    },
    Completed(Vec<(f32, Keyboard)>),
}

impl AppState {
    pub fn new_job(batch_size: usize) -> Self {
        Self::Running {
            batch_size,
            completed: 0,
        }
    }
    
    pub fn add_one_completed(&mut self) {
        if let Self::Running { completed, .. } = self {
            *completed += 1;
        }
    }
}


#![allow(dead_code)]

use keyboard_gen::factory::{start_generation, Keyboard};
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use axum::{Json, Router, routing::{post, get}, extract::State};
use serde::{Serialize, Deserialize};
use keyboard_gen::AppState;

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let state = Arc::new(Mutex::new(AppState::Init));

    let router = Router::new()
        .route("/new", post(start_batch))
        .route("/update", get(update))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 5000));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}

#[axum::debug_handler]
async fn start_batch(
    State(shared_state): State<SharedState>,
    Json(BatchRequest{ job_name, batch_size }): Json<BatchRequest>,
) -> Json<Response> {
    let mut state = shared_state.lock().unwrap();
    let resp = match state.clone() {
        AppState::Init =>
            Json(Response::Init { message: RespMsg::Init }),
        AppState::Running { batch_size, completed } =>
            return Json(Response::InProgress {
                batch_size,
                completed,
                message: RespMsg::InProgress
            }),
        AppState::Completed(keyboard_vec) =>
            Json(Response::BatchComplete {
                message: RespMsg::Completed,
                keyboards: keyboard_vec
                    .iter()
                    .map(|&kb| kb.into())
                    .collect()
            }),
    };
    
    *state = AppState::new_job(batch_size);
    
    let thread_state = shared_state.clone();
    tokio::spawn(async move {
        start_generation(job_name, batch_size, thread_state);
    });
    resp
}

async fn update(State(shared_state): State<SharedState>) -> Json<Response> {
    let state = shared_state.lock().unwrap();
    match state.clone() {
        AppState::Init =>
            Json(Response::Init { message: RespMsg::Init }),
        AppState::Running { batch_size, completed } =>
            Json(Response::InProgress {
                batch_size,
                completed,
                message: RespMsg::InProgress,
            }),
        AppState::Completed(keyboard_vec) =>
            Json(Response::BatchComplete {
                message: RespMsg::Completed,
                keyboards: keyboard_vec
                    .iter()
                    .map(|&kb| kb.into())
                    .collect()
                })
    }
}

#[derive(Serialize)]
enum RespMsg {
    Init,
    InProgress,
    Completed,
}

#[derive(Deserialize)]
struct BatchRequest {
    job_name: String,
    batch_size: usize,
}

#[derive(Serialize)]
enum Response {
    Init {
        message: RespMsg,
    },
    InProgress {
        message: RespMsg,
        batch_size: usize,
        completed: usize,
    },
    BatchComplete {
        message: RespMsg,
        keyboards: Vec<KeyboardResp>,
    },
}

#[derive(Serialize)]
struct KeyboardResp {
    score: f32,
    keyboard: Keyboard
}

impl From<(f32, Keyboard)> for KeyboardResp {
    fn from((score, keyboard): (f32, Keyboard)) -> Self {
        Self { score, keyboard }
    }
}

#[derive(Serialize)]
struct Results(Vec<Keyboard>);


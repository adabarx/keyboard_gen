#![allow(dead_code)]

use keyboard_gen::factory::{go, Keyboard};
use std::{net::SocketAddr, sync::{Arc, Mutex}};
use axum::{Json, Router, http::StatusCode, routing::{post, get}, extract::State};
use serde::{Serialize, Deserialize};
use keyboard_gen::AppState;

type SharedState = Arc<Mutex<AppState>>;

#[tokio::main]
async fn main() {
    let state = Arc::new(Mutex::new(AppState::Init));

    let router = Router::new()
        .route("/new", post(start_batch))
        .route("/update", get(update))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(router.into_make_service())
        .await
        .unwrap();
}

#[axum::debug_handler]
async fn start_batch(
    State(shared_state): State<SharedState>,
    Json(BatchRequest{ batch_size }): Json<BatchRequest>,
) -> (StatusCode, Json<BatchStartResponse>) {
    let mut state = shared_state.lock().unwrap();
    let resp = match state.clone() {
        AppState::Init => (StatusCode::OK, Json(BatchStartResponse::Success(None))),
        AppState::Running { batch_size, completed } => return (
            StatusCode::CONFLICT,
            Json(BatchStartResponse::BatchInProgress { batch_size, completed })
        ),
        AppState::Completed(keyboard_vec) =>
            (StatusCode::OK, Json(BatchStartResponse::Success(Some(keyboard_vec.clone())))),
    };
    
    *state = AppState::new_job(batch_size);
    
    let thread_state = shared_state.clone();
    tokio::spawn(async move {
        go(batch_size, thread_state);
    });
    resp
}

async fn update(State(shared_state): State<SharedState>) 
    -> (StatusCode, Json<UpdateResponse>)
{
    let state = shared_state.lock().unwrap();
    match state.clone() {
        AppState::Init => (StatusCode::OK, Json(UpdateResponse::NoBatch)),
        AppState::Running { batch_size, completed } =>
            (StatusCode::OK, Json(UpdateResponse::BatchInProgress { batch_size, completed })),
        AppState::Completed(keyboard_vec) =>
            (StatusCode::OK, Json(UpdateResponse::BatchComplete(keyboard_vec))),
    }
}

#[derive(Deserialize)]
struct BatchRequest {
    batch_size: usize,
}

#[derive(Serialize)]
enum BatchStartResponse {
    Success(Option<Vec<(f32, Keyboard)>>),
    BatchInProgress {
        batch_size: usize,
        completed: usize,
    }
}

#[derive(Serialize)]
enum UpdateResponse {
    NoBatch,
    BatchInProgress {
        batch_size: usize,
        completed: usize,
    },
    BatchComplete(Vec<(f32, Keyboard)>)
}

#[derive(Serialize)]
struct Results(Vec<Keyboard>);

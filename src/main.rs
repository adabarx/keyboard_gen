#![allow(dead_code)]
use axum::Json;
use hyper::StatusCode;
use keyboard_gen::{go, Keyboard};
use serde::{Serialize, Deserialize};

#[tokio::main]
async fn main() {
    go();
}

async fn start_batch(Json(_payload): Json<BatchRequest>) 
    -> (StatusCode, BatchStartResponse)
{
    todo!("start an arbitrary sized batch of keyboards and return a confirmation")
}

async fn update() 
    -> (StatusCode, UpdateResponse)
{
    todo!("return the status of the batch in progress")
}

#[derive(Deserialize)]
struct BatchRequest(usize);

#[derive(Serialize)]
enum BatchStartResponse {
    Success,
    BatchInProgress,
    Failed,
}

#[derive(Serialize)]
enum UpdateResponse {
    NoBatch,
    BatchInProgress {
        complete: usize,
        size: usize,
    },
    BatchComplete(Vec<Keyboard>)
}

#[derive(Serialize)]
struct Results(Vec<Keyboard>);

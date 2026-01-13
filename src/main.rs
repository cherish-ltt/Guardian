mod controller;
mod dto;
mod entities;
mod error;
mod middleware;
mod response;
mod router;
mod service;
mod utils;

use crate::router::get_router;
use axum::Router;
use dotenv::dotenv;
use log::info;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();
    
    let app = get_router().await.unwrap();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:6123").await.unwrap();
    info!("listening to 0.0.0.0:6123");
    axum::serve(listener, app).await.unwrap();
}

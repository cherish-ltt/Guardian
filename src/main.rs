mod controller;
mod dto;
mod entities;
mod error;
mod middleware;
mod response;
mod router;
mod service;
mod utils;

use crate::{router::get_router, utils::hash_password};
use axum::Router;
use dotenv::dotenv;
use log::info;

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::fmt().init();
    println!("{}",hash_password("123456"));

    let app = get_router().await.unwrap();

    let host = std::env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("SERVER_PORT")
        .unwrap_or_else(|_| "6123".to_string())
        .parse::<u16>()
        .unwrap_or(6123);

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    info!("listening to {}", addr);
    axum::serve(listener, app).await.unwrap();
}

#![allow(unused_imports)] // TODO delete

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

use redis::Commands;
use std::net::SocketAddr;

use tokio::net::TcpListener;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Unable to load .env file");

    let server_address = std::env::var("SERVER_ADDRESS").expect("SERVER_ADDRESS not defined");

    // TODO connect to redis DB
    match connect_to_redis().await {
        Ok(value) => println!("Connected to Redis: {}", value),
        Err(err) => println!("Error connecting to Redis: {}", err),
    }

    let listener = TcpListener::bind(server_address).await.unwrap();
    println!("Listening on: {}", listener.local_addr().unwrap());

    let app = Router::new().route("/", get(|| async {
        "Hello, World!"
    }));

    axum::serve(listener, app).await.expect("Error serving application");
}

async fn connect_to_redis() -> redis::RedisResult<String> {
    // TODO error handling
    
    // TODO .env
    let client = redis::Client::open("redis://database:6379/")?;
    let mut con = client.get_connection()?;

    con.set("test_key", "Hello from API-Dev")?;
    con.get("test_key")
}
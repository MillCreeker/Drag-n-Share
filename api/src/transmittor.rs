#[allow(unused_imports)] // TODO

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};

use tokio::net::TcpListener;

use axum_client_ip::{SecureClientIp, SecureClientIpSource};

// use serde::{Deserialize, Serialize};
use serde_json::json;

use redis::aio::ConnectionManager;

use std::net::SocketAddr;

use env_logger::Env;
use log::info;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    dotenvy::dotenv().expect("Unable to load .env file");

    let redis_connection_manager: ConnectionManager = utils::get_redis_connection_manager()
        .await
        .expect("Error connecting to Redis");
    info!("Connected to Redis");

    let listener = TcpListener::bind("0.0.0.0:7879").await.unwrap();
    info!("Listening on: {}", listener.local_addr().unwrap());

    let app = Router::new()
        .route("/", get(ping))
        .with_state(redis_connection_manager)
        .layer(SecureClientIpSource::ConnectInfo.into_extension());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Error serving application");
}

async fn ping(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm, &secure_ip).await?;

    let timestamp = utils::get_current_timestamp();

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": timestamp
        })
        .to_string(),
    ))
}
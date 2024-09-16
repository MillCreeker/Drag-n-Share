#[allow(unused_imports)] // TODO
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::sse::{Event, KeepAlive, Sse},
    routing::get,
    Json, Router,
};

use tokio::net::TcpListener;

use futures_util::stream;
use std::time::Duration;
use tokio::time;
use axum::response::IntoResponse;

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
        .route("/:session_id/:file_name", get(get_file))
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

async fn get_file(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path((session_id, file_name)): Path<(String, String)>,
) -> impl IntoResponse {
    // Number of events to send before closing the connection
    let max_events = 5;

    // Create a stream of events
    let stream = stream::unfold(0, move |count| async move {
        if count >= max_events {
            // Return None to stop the stream and close the connection
            None
        } else {
            // Simulate an event every second
            time::sleep(Duration::from_secs(1)).await;
            
            // Create an event with data, wrap it in Ok to match the required type
            let event: Result<Event, String> = Ok(Event::default().data(format!("event number: {}", count)));
            
            // Continue the stream
            Some((event, count + 1))
        }
    });

    // Convert the stream into an SSE response
    Sse::new(stream)
}

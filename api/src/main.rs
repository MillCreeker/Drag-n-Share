#![allow(unused_imports)] // TODO delete

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch},
    Json, Router,
};

use tokio::net::TcpListener;

use axum_client_ip::{SecureClientIp, SecureClientIpSource};

use serde::{Deserialize, Serialize};
use serde_json::json;

use redis::{aio::ConnectionManager, streams::StreamAutoClaimOptions, AsyncCommands, Commands};

use std::{fmt::Debug, net::SocketAddr};

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Unable to load .env file");

    let redis_connection_manager: ConnectionManager = get_redis_connection_manager()
        .await
        .expect("Error connecting to Redis");
    println!("Connected to Redis");

    let server_address = std::env::var("BASE_URL").expect("URL_BASE not defined");
    let listener = TcpListener::bind(server_address).await.unwrap();
    println!("Listening on: {}", listener.local_addr().unwrap());

    let app = Router::new()
        .route("/", get(ping))
        .route("/session", get(ping).post(create_session))
        .route(
            "/session/:session_name",
            get(get_session).put(update_session),
        )
        .with_state(redis_connection_manager)
        .layer(SecureClientIpSource::ConnectInfo.into_extension());

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .expect("Error serving application");
}

async fn get_redis_connection_manager() -> Result<redis::aio::ConnectionManager, redis::RedisError>
{
    let database_host = std::env::var("DATABASE_HOST").expect("DATABASE_HOST not defined");
    let database_password = std::env::var("DATABASE_PASSWORD").unwrap_or_default();

    let redis_conn_url = format!("redis://:{}@{}", database_password, database_host);
    let client = redis::Client::open(redis_conn_url)?;

    let config = redis::aio::ConnectionManagerConfig::new();

    let redis_connection_manager = match ConnectionManager::new_with_config(client, config).await {
        Ok(m) => m,
        Err(e) => {
            println!("Error connecting to Redis: {}", e);
            return Err(e);
        }
    };

    Ok(redis_connection_manager)
}

const CALL_RATE_LIMIT_SEC: u64 = 1;

async fn handle_call_rate_limit(
    mut rcm: State<ConnectionManager>,
    ip: SecureClientIp,
) -> Result<bool, (StatusCode, String)> {
    let ref key = format!("call-{}", ip.0.to_string());

    match rcm
        .get_ex::<&String, String>(&key, redis::Expiry::EX(CALL_RATE_LIMIT_SEC))
        .await
    {
        Ok(_) => Err((
            StatusCode::TOO_MANY_REQUESTS,
            json!({
                "success": false,
                "message": "rate limit exceeded"
            })
            .to_string(),
        )),
        Err(_) => {
            rcm.set_ex::<&String, bool, bool>(&key, true, CALL_RATE_LIMIT_SEC)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        json!({
                            "success": false,
                            "message": e.to_string()
                        })
                        .to_string(),
                    )
                })?;

            Ok(true)
        }
    }
}

async fn ping(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    handle_call_rate_limit(rcm, secure_ip).await?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": timestamp
        })
        .to_string(),
    ))
}

#[derive(Serialize)]
struct File {
    filename: String,
    size: u64,
    owner: String,
}

#[derive(Serialize)]
struct Session {
    name: String,
    timestamp: u128,
    files: Vec<File>,
}

async fn get_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Path(session_name): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    handle_call_rate_limit(rcm, secure_ip).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": session_name
        })
        .to_string(),
    ))
}

async fn get_random_dragon_name(
    mut rcm: State<ConnectionManager>,
) -> Result<String, (StatusCode, String)> {
    let dragon_names = vec![
        "Smaug",
        "Drogon",
        "Slifer",
        "Tiamat",
        "Toothless",
        "Drake",
        "Dragonite",
        "Viserion",
        "Draco",
        "Falkor",
        "Saphira",
        "Mushu",
        "Diaval",
        "Haku",
        "Rhaegal",
        "Balerion",
        "Meraxes",
        "Syrax",
    ];

    // first random name
    let dragon_name = dragon_names.choose(&mut rand::thread_rng()).unwrap_or(&dragon_names[0]).to_string();
    let key = format!("session-{}", dragon_name);

    if !rcm.exists::<&String, bool>(&key).await.unwrap_or(false) {
        return Ok(dragon_name);
    }

    rcm.get_ex::<&String, String>(&key, redis::Expiry::EX(CALL_RATE_LIMIT_SEC))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": e.to_string()
                })
                .to_string(),
            )
        })?;

    // any name from list
    for name in dragon_names {
        let key = format!("session-{}", name);
        if !rcm.exists::<String, bool>(key).await.unwrap_or(false) {
            return Ok(name.to_string());
        }
    }

    // first random name with counter
    let mut counter = 1;
    loop {
        let nr_key = format!("{}{}", &key, counter);

        if !rcm.exists::<&String, bool>(&nr_key).await.unwrap_or(false) {
            return Ok(format!("{}{}", &key, counter));
        }

        counter += 1;
    }
}

async fn create_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    handle_call_rate_limit(rcm.clone(), secure_ip).await?;

    let name = get_random_dragon_name(rcm).await?;

    let session = Session {
        name: name.clone(),
        timestamp: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
        files: vec![],
    };

    Ok((
        StatusCode::CREATED,
        json!({
            "success": true,
            "response": session
        })
        .to_string(),
    ))
}

async fn update_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Path(session_name): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    handle_call_rate_limit(rcm, secure_ip).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": session_name
        })
        .to_string(),
    ))
}

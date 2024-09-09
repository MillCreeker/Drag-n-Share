#![allow(unused_imports)] // TODO delete

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, patch},
    Json, Router,
};

use tokio::net::TcpListener;

use axum_client_ip::{SecureClientIp, SecureClientIpSource};

use serde::{Deserialize, Serialize};
use serde_json::json;

use redis::{aio::ConnectionManager, streams::StreamAutoClaimOptions, AsyncCommands, Commands};
use utils::handle_call_rate_limit;

use std::{fmt::Debug, net::SocketAddr};

use std::time::{SystemTime, UNIX_EPOCH};

use rand::seq::SliceRandom;

mod redis_handler;
mod utils;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().expect("Unable to load .env file");

    let redis_connection_manager: ConnectionManager = utils::get_redis_connection_manager()
        .await
        .expect("Error connecting to Redis");
    println!("Connected to Redis");

    let listener = TcpListener::bind("0.0.0.0:7878").await.unwrap();
    println!("Listening on: {}", listener.local_addr().unwrap());

    let app = Router::new()
        .route("/", get(ping))
        .route("/session", get(does_session_exist).post(create_session))
        .route(
            "/session/:session_name",
            get(get_session).options(join_session).put(update_session),
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

async fn ping(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm, &secure_ip).await?;

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

async fn does_session_exist(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Json(session_name): Json<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let key = format!("session:{}", session_name);
    let exists = redis_handler::exists(rcm, &key).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "doesSessionExist": exists
        })
        .to_string(),
    ))
}

async fn get_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_name): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let key = format!("session:{}", session_name);
    if !redis_handler::exists(rcm, &key).await? {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "session name not found"
            })
            .to_string(),
        ));
    }

    let auth = headers.get("Authorization");
    if auth.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({
                "success": false,
                "message": "authorization header not found"
            })
            .to_string(),
        ));
    }

    let auth = auth.unwrap().to_str().unwrap();

    // TODO get session

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": auth
        })
        .to_string(),
    ))
}

async fn join_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_name): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let key = format!("session:{}", session_name);
    if !redis_handler::exists(rcm, &key).await? {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "session name not found"
            })
            .to_string(),
        ));
    }

    let auth = headers.get("Authorization");
    if auth.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({
                "success": false,
                "message": "authorization header not found"
            })
            .to_string(),
        ));
    }

    let auth = auth.unwrap().to_str().unwrap();

    // TODO generate access token

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": auth
        })
        .to_string(),
    ))
}

#[derive(Deserialize)]
struct FileReq {
    name: String,
    size: u64,
}

// #[derive(Serialize)]
// struct File {
//     name: String,
//     size: u64,
//     owner_ip: String,
// }

// #[derive(Serialize)]
// struct Session {
//     name: String,
//     owner_ip: String,
//     files: Vec<File>,
// }

async fn create_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Json(file_metadata): Json<Vec<FileReq>>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let session_name = utils::get_random_dragon_name(rcm.clone()).await?;

    // TODO save file metadata

    let access_token = utils::get_random_access_token();
    let access_code = utils::get_random_six_digit_code();
    let uuid = utils::get_uuid();
    let encrypetd_token = utils::sha256(&access_token);
    let encrypetd_code = utils::sha256(&access_code);

    let items = [
        ("code", encrypetd_code.as_str()),
        ("owner.id", uuid.as_str()),
    ];

    let session_key = format!("session:{}", session_name);
    redis_handler::hset_multiple(rcm.clone(), &session_key, &items, None).await?;

    let token_key = format!("session:{}:{}", encrypetd_token, uuid);
    redis_handler::set(rcm.clone(), &token_key, encrypetd_token.as_str(), None).await?;

    for file in file_metadata {
        let session_file_key = format!("files:{}", session_name);
        redis_handler::set(rcm.clone(), &session_file_key, file.name.as_str(), None).await?;

        let file_key = format!("file:{}:{}", session_name, file.name);

        let size_string = file.size.to_string();
        let items = [
            ("name", file.name.as_str()),
            ("size", size_string.as_str()),
            ("owner.id", uuid.as_str()),
        ];
        redis_handler::hset_multiple(rcm.clone(), &file_key, &items, None).await?;
    }

    Ok((
        StatusCode::CREATED,
        json!({
            "success": true,
            "response": {
                "sessionName": session_name,
                "token": access_token,
                "code": access_code,
                "uuid": uuid,
            }
        })
        .to_string(),
    ))
}

async fn update_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Path(session_name): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm, &secure_ip).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": session_name
        })
        .to_string(),
    ))
}

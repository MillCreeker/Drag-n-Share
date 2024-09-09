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

use std::{
    fmt::{format, Debug},
    net::SocketAddr,
};

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
            get(get_files_metadata)
                .options(join_session)
                .delete(delete_session),
        )
        .route(
            "/session/:session_name/:file_name",
            get(get_file_metadata).post(add_file).delete(remove_file),
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

#[derive(Serialize)]
struct File {
    name: String,
    size: u64,
    owner_id: String,
}

async fn get_files_metadata(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_name): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_name).await?;

    utils::check_is_user_authorized(rcm.clone(), &headers, &session_name).await?;

    let file_set_key = format!("files:{}", session_name);
    let files = redis_handler::smembers(rcm.clone(), &file_set_key).await?;

    let mut file_metadata: Vec<File> = vec![];

    for file in files {
        let file_key = format!("file:{}:{}", session_name, file);

        let metadata = redis_handler::hgetall(rcm.clone(), &file_key).await?;
        file_metadata.push(File {
            name: file,
            size: metadata[3].parse::<u64>().unwrap_or(0),
            owner_id: metadata[5].to_string(),
        });
    }

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": file_metadata
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
    utils::check_session_exists(rcm.clone(), &session_name).await?;

    let auth = utils::get_header(&headers, "authorization")?;

    let encr_code = utils::sha256(auth.as_str());

    let key = format!("session:{}", session_name);
    let ret = redis_handler::hgetall(rcm.clone(), &key).await?;

    let actual_code = ret[1].clone();
    if actual_code != encr_code {
        return Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "message": "authorization code invalid"
            })
            .to_string(),
        ));
    }

    let uuid = utils::get_uuid();
    let token = utils::get_random_access_token();
    let encrypetd_token = utils::sha256(&token);

    let token_key = format!("session:{}:{}", session_name, uuid);
    redis_handler::set(rcm, &token_key, encrypetd_token.as_str(), None).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": {
                "uuid": uuid,
                "token": token,
            }
        })
        .to_string(),
    ))
}

#[derive(Deserialize)]
struct FileReq {
    name: String,
    size: u64,
}

async fn create_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Json(file_metadata): Json<Vec<FileReq>>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let session_name = utils::get_random_dragon_name(rcm.clone()).await?;

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

    let token_key = format!("session:{}:{}", session_name, uuid);
    redis_handler::set(rcm.clone(), &token_key, encrypetd_token.as_str(), None).await?;

    for file in file_metadata {
        let session_file_key = format!("files:{}", session_name);
        redis_handler::sadd(rcm.clone(), &session_file_key, file.name.as_str(), None).await?;

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
                "uuid": uuid,
                "token": access_token,
                "code": access_code,
            }
        })
        .to_string(),
    ))
}

// TODO
async fn delete_session(
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

// TODO
async fn get_file_metadata(
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

// TODO
async fn add_file(
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

// TODO
async fn remove_file(
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

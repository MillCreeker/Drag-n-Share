use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};

use tokio::net::TcpListener;

use axum_client_ip::{SecureClientIp, SecureClientIpSource};
use http::header::HeaderValue;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use serde::{Deserialize, Serialize};
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

    let cors = CorsLayer::new()
        .allow_origin(Any)
        // .allow_origin(AllowOrigin::exact(
        //     HeaderValue::from_str("https://drag-n-share.com").unwrap(),
        // )) // TODO
        .allow_methods(Any)
        .allow_headers(Any);

    let listener = TcpListener::bind("0.0.0.0:7878").await.unwrap();
    info!("Listening on: {}", listener.local_addr().unwrap());

    let app = Router::new()
        .route("/", get(ping))
        .route("/session", get(get_session).post(create_session))
        .route("/idForName/:session_name", get(get_id_for_session_name))
        .route("/access/:session_id", get(join_session))
        .route(
            "/session/:session_id",
            get(get_session_metadata)
                .put(update_session)
                .delete(delete_session),
        )
        .route(
            "/files/:session_id",
            get(get_all_file_metadata_in_session).post(add_files),
        )
        .route(
            "/files/:session_id/:file_name",
            get(get_file_metadata).delete(delete_file),
        )
        .with_state(redis_connection_manager)
        .layer(cors)
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

async fn get_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let claims = utils::decode_jwt_from_header(&headers)?;
    let session_id = claims.aud;

    utils::check_session_exists(rcm.clone(), &session_id).await?;
    utils::check_user_is_host(&headers, &session_id)?;

    let key = format!("session:{}", session_id);
    let session_name = utils::redis_handler::hget(rcm.clone(), &key, "name").await?;

    let code = utils::get_random_six_digit_code();
    let encrypted_code = utils::sha256(&code);

    let items = [
        ("name", session_name.as_str()),
        ("code", encrypted_code.as_str()),
    ];
    utils::redis_handler::hset_multiple(rcm.clone(), &key, &items, None).await?;

    let key = format!("session:{}", &session_name);
    utils::redis_handler::set(rcm, &key, &session_id, None).await?;

    Ok((
        StatusCode::ACCEPTED,
        json!({
            "success": true,
            "response": {
                "sessionName": session_name,
                "sessionId": session_id,
                "accessCode": code
            }
        })
        .to_string(),
    ))
}

async fn create_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let key = format!("created.sessions:{}", &secure_ip.0);
    if utils::redis_handler::exists(rcm.clone(), &key).await? {
        return Err((
            StatusCode::CONFLICT,
            json!({
                "success": false,
                "message": "you have already created a session"
            })
            .to_string(),
        ));
    }

    let session_name = utils::get_random_dragon_name(rcm.clone()).await?;
    let session_id = utils::get_uuid();
    let user_id = utils::get_uuid();
    let jwt = utils::create_jwt(&session_id, Some(&user_id))?;

    let code = utils::get_random_six_digit_code();
    let encrypted_code = utils::sha256(&code);

    let key = format!("session:{}", session_name);
    utils::redis_handler::set(rcm.clone(), &key, &session_id, None).await?;

    let key = format!("session:{}", session_id);
    let items = [("name", session_name.as_str()), ("code", &encrypted_code)];
    utils::redis_handler::hset_multiple(rcm.clone(), &key, &items, None).await?;

    let key = format!("created.sessions:{}", &secure_ip.0);
    utils::redis_handler::set(rcm, &key, &session_id, None).await?;

    Ok((
        StatusCode::CREATED,
        json!({
            "success": true,
            "response": {
                "sessionName": session_name,
                "sessionId": session_id,
                "accessCode": code,
                "jwt": jwt
            }
        })
        .to_string(),
    ))
}

async fn get_id_for_session_name(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Path(session_name): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;

    let key = format!("session:{}", session_name);

    if !utils::redis_handler::exists(rcm.clone(), &key).await? {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "session name not found"
            })
            .to_string(),
        ));
    }

    let session_id = utils::redis_handler::get(rcm, &key).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": {
                "sessionId": session_id,
            }
        })
        .to_string(),
    ))
}

async fn get_session_metadata(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Path(session_id): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;

    let key = format!("session:{}", session_id);
    let session_name = utils::redis_handler::hget(rcm.clone(), &key, "name").await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": {
                "sessionName": session_name
            }
        })
        .to_string(),
    ))
}

async fn join_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    info!("Testing");
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;
    let key = format!("access.attempts:{}:{}", session_id, secure_ip.0);
    if utils::redis_handler::get(rcm.clone(), &key).await? == "5" {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            json!({
                "success": false,
                "message": "too many attempts"
            })
            .to_string(),
        ));
    }

    let encrypted_code = utils::get_header(&headers, "authorization")?;

    let key = format!("session:{}", session_id);
    let code = utils::redis_handler::hget(rcm.clone(), &key, "code").await?;

    if encrypted_code != code {
        let key = format!("access.attempts:{}:{}", session_id, secure_ip.0);
        utils::redis_handler::incr(rcm, &key, Some(10)).await?;

        return Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "message": "invalid access code"
            })
            .to_string(),
        ));
    }

    let jwt = utils::create_jwt(&session_id, None)?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": {
                "jwt": jwt
            }
        })
        .to_string(),
    ))
}

#[derive(Deserialize)]
struct SessionNameBody {
    name: String,
}

async fn update_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(session_name_body): Json<SessionNameBody>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;

    utils::check_user_is_host(&headers, &session_id)?;

    let key = format!("session:{}", session_id);
    let old_session_name = utils::redis_handler::hget(rcm.clone(), &key, "name").await?;

    let code = utils::get_random_six_digit_code();
    let encrypted_code = utils::sha256(&code);

    let new_name = session_name_body.name;
    let items = [
        ("name", new_name.as_str()),
        ("code", encrypted_code.as_str()),
    ];
    utils::redis_handler::hset_multiple(rcm.clone(), &key, &items, None).await?;

    let key = format!("session:{}", old_session_name);
    utils::redis_handler::del(rcm.clone(), &key).await?;

    let key = format!("session:{}", &new_name);
    utils::redis_handler::set(rcm, &key, &session_id, None).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": {
                "accessCode": code
            }
        })
        .to_string(),
    ))
}

async fn delete_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;

    utils::check_user_is_host(&headers, &session_id)?;

    let key = format!("created.sessions:{}", secure_ip.0);
    utils::redis_handler::del(rcm.clone(), &key).await?;

    let key = format!("session:{}", session_id);
    let session_name = utils::redis_handler::hget(rcm.clone(), &key, "name").await?;
    utils::redis_handler::del(rcm.clone(), &key).await?;

    let key = format!("session:{}", session_name);
    utils::redis_handler::del(rcm.clone(), &key).await?;

    let key = format!("files:{}", session_id);
    let files = utils::redis_handler::smembers(rcm.clone(), &key).await?;
    for file in files {
        let key = format!("files:{}:{}", session_id, file);
        utils::redis_handler::del(rcm.clone(), &key).await?;
    }

    let key = format!("files:{}", session_id);
    utils::redis_handler::del(rcm, &key).await?;

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": "successfully deleted session"
        })
        .to_string(),
    ))
}

#[derive(Serialize)]
struct FileMetadataResponse {
    name: String,
    size: u64,
    is_owner: bool,
}

async fn get_all_file_metadata_in_session(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;

    let user = utils::check_user_is_in_session(&headers, &session_id)?;

    let mut files: Vec<FileMetadataResponse> = Vec::new();

    let key = format!("files:{}", session_id);
    let file_names = utils::redis_handler::smembers(rcm.clone(), &key).await?;

    for file_name in file_names {
        let file = utils::redis_handler::hgetall(
            rcm.clone(),
            &format!("files:{}:{}", &session_id, &file_name),
        )
        .await?;

        if file.len() != 6 {
            continue;
        }
        let file = FileMetadataResponse {
            name: file[1].clone(),
            size: file[3].parse().unwrap_or(0),
            is_owner: file[5] == user.id,
        };
        files.push(file);
    }

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": files
        })
        .to_string(),
    ))
}

struct FileMetadata {
    name: String,
    size: u64,
    owner_id: String,
}

#[derive(Deserialize)]
struct FileMetadataBody {
    name: String,
    size: u64,
}

async fn add_files(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Json(files): Json<Vec<FileMetadataBody>>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;

    let user = utils::check_user_is_in_session(&headers, &session_id)?;

    let mut new_files: Vec<FileMetadata> = Vec::new();

    let key = format!("files:{}", session_id);

    for file in files {
        if utils::redis_handler::sismember(rcm.clone(), &key, &file.name).await? {
            return Err((
                StatusCode::BAD_REQUEST,
                json!({
                    "success": false,
                    "response": {
                        "message": format!("file \"{}\" already exists", &file.name),
                        "file": &file.name
                    }
                })
                .to_string(),
            ));
        }

        new_files.push(FileMetadata {
            name: file.name,
            size: file.size,
            owner_id: user.id.clone(),
        });
    }

    if new_files.len() == 0 {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({
                "success": false,
                "response": {
                    "message": "no files provided"
                }
            })
            .to_string(),
        ));
    }

    for file in new_files {
        let key = format!("files:{}:{}", &session_id, &file.name);
        let file_size = file.size.to_string();
        let items = [
            ("name", file.name.as_str()),
            ("size", file_size.as_str()),
            ("owner.id", file.owner_id.as_str()),
        ];
        utils::redis_handler::hset_multiple(rcm.clone(), &key, &items, None).await?;

        let key = format!("files:{}", &session_id);
        utils::redis_handler::sadd(rcm.clone(), &key, &file.name, None).await?;
    }

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": "successfully added files"
        })
        .to_string(),
    ))
}

async fn get_file_metadata(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path((session_id, file_name)): Path<(String, String)>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;

    let user = utils::check_user_is_in_session(&headers, &session_id)?;

    let key = format!("files:{}:{}", &session_id, &file_name);
    if !utils::redis_handler::exists(rcm.clone(), &key).await? {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "file not found"
            })
            .to_string(),
        ));
    }

    let file_data = utils::redis_handler::hgetall(rcm, &key).await?;

    if file_data.len() != 6 {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "file not found"
            })
            .to_string(),
        ));
    }

    let file = FileMetadataResponse {
        name: file_data[1].clone(),
        size: file_data[3].parse().unwrap_or(0),
        is_owner: file_data[5] == user.id,
    };

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": file
        })
        .to_string(),
    ))
}

async fn delete_file(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    headers: HeaderMap,
    Path((session_id, file_name)): Path<(String, String)>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // utils::handle_call_rate_limit(rcm.clone(), &secure_ip).await?;
    utils::check_session_exists(rcm.clone(), &session_id).await?;

    let user = utils::check_user_is_in_session(&headers, &session_id)?;

    let key = format!("files:{}:{}", &session_id, &file_name);
    if !utils::redis_handler::exists(rcm.clone(), &key).await? {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "file not found"
            })
            .to_string(),
        ));
    }

    let user_id = utils::redis_handler::hget(rcm.clone(), &key, "owner.id").await?;

    if user.id == user_id || user.is_host {
        utils::redis_handler::del(rcm.clone(), &key).await?;

        let key = format!("files:{}", &session_id);
        utils::redis_handler::srem(rcm, &key, &file_name).await?;
    } else {
        return Err((
            StatusCode::FORBIDDEN,
            json!({
                "success": false,
                "message": "you are not allowed to delete this file"
            })
            .to_string(),
        ));
    }

    Ok((
        StatusCode::OK,
        json!({
            "success": true,
            "response": "successfully deleted file"
        })
        .to_string(),
    ))
}

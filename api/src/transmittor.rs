use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

use tokio::net::TcpListener;

use once_cell::sync::Lazy;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::time;
use tokio_stream::StreamExt;

use dashmap::DashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use axum_client_ip::SecureClientIpSource;

use tower_http::cors::{Any, CorsLayer};

use redis::aio::ConnectionManager;

use env_logger::Env;
use log::{error, info};

const MAX_CHUNK_SIZE: usize = 70_000;

static LISTENERS: Lazy<Arc<DashMap<String, ()>>> = Lazy::new(|| Arc::new(DashMap::new()));

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
        .allow_methods(Any)
        .allow_headers(Any);

    let listener = TcpListener::bind("0.0.0.0:7879").await.unwrap();
    info!("Listening on: {}", listener.local_addr().unwrap());

    let app = Router::new()
        .route("/session/:session_id", get(ws_handler))
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

#[derive(Deserialize)]
struct Request {
    jwt: String,
    command: String,
    data: String,
}

async fn ws_handler(
    rcm: State<ConnectionManager>,
    // secure_ip: SecureClientIp,
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!("Websocket connection opened");
    ws.on_upgrade(|ws| ws_handler_inner(rcm, session_id, ws))
}

async fn ws_handler_inner(
    rcm: State<ConnectionManager>,
    // secure_ip: SecureClientIp,
    session_id: String,
    mut socket: WebSocket,
) {
    let (tx, mut rx) = mpsc::channel::<String>(1024);
    let (shutdown_tx, shutdown_rx) = watch::channel(());
    // Use tokio::select! to handle reading from the WebSocket and sending messages concurrently

    let tx_clone = tx.clone();
    let shutdown_rx_clone = shutdown_rx.clone();
    loop {
        tokio::select! {
            // Handle receiving from the WebSocket
            message = socket.next() => {
                match message {
                    Some(Ok(Message::Text(text))) => {
                        // info!("Received message from client: {}", text);

                        handle_incomming_message(
                            tx_clone.clone(),
                            shutdown_rx_clone.clone(),
                            rcm.clone(),
                            &session_id,
                            &text,
                        ).await;
                    }
                    Some(Ok(_)) => {} // Handle other message types if needed
                    Some(Err(e)) => {
                        error!("Error reading from socket: {}", e);
                        break;
                    }
                    None => {
                        error!("Client disconnected");
                        break;
                    }
                }
            }

            // Handle sending messages from the channel to the WebSocket
            Some(message) = rx.recv() => {
                if socket.send(Message::Text(message)).await.is_err() {
                    error!("Client disconnected");
                    break;
                }
            }
        }
    }

    shutdown_tx.send(()).unwrap();
    info!("Websocket connection closed");
}

async fn handle_incomming_message(
    tx: mpsc::Sender<String>,
    shutdown_rx: watch::Receiver<()>,
    rcm: State<ConnectionManager>,
    session_id: &String,
    message: &str,
) {
    match serde_json::from_str::<Request>(message) {
        Ok(request) => {
            let user_id: Option<String> = match utils::decode_jwt(&request.jwt) {
                Ok(claims) => {
                    if claims.aud != session_id.as_str() {
                        error!("Invalid session ID");
                        None
                    } else {
                        Some(claims.sub)
                    }
                }
                Err(_) => {
                    error!("Failed to decode JWT: {}", request.jwt);
                    None
                }
            };

            // handle commands
            let response = if user_id.is_none() {
                Err((
                    StatusCode::BAD_REQUEST,
                    "No user ID found in JWT.".to_string(),
                ))
            } else if request.command == "register" {
                start_listeners(
                    tx,
                    shutdown_rx,
                    rcm.clone(),
                    &session_id,
                    &user_id.unwrap_or("".to_string()),
                )
                .await
            } else if request.command == "request-file" {
                request_file(
                    rcm.clone(),
                    session_id,
                    &user_id.unwrap_or("".to_string()),
                    &request.data,
                )
                .await
            } else if request.command == "acknowledge-file-request" {
                acknowledge_file_request(rcm.clone(), &request.data).await
            } else if request.command == "ready-for-file-transfer" {
                ready_for_file_transfer(
                    rcm.clone(),
                    &user_id.unwrap_or("".to_string()),
                    &request.data,
                )
                .await
            } else if request.command == "add-chunk" {
                add_chunk(
                    rcm.clone(),
                    &user_id.unwrap_or("".to_string()),
                    &request.data,
                )
                .await
            } else if request.command == "received-chunk" {
                received_chunk(
                    rcm.clone(),
                    &user_id.unwrap_or("".to_string()),
                    &request.data,
                )
                .await
            } else {
                Err((
                    StatusCode::BAD_REQUEST,
                    format!("Unknown command: {}", request.command),
                ))
            };

            if let Err((status, message)) = response {
                error!("{} - {}", status.as_u16(), message);
            }
        }
        Err(e) => error!("Failed to deserialize incoming message: {}", e),
    }
}

trait WsMsgData {}

#[derive(Serialize)]
struct WsMessage<T: WsMsgData> {
    request_id: String,
    command: String,
    data: T,
}

#[derive(Serialize)]
struct WsMsgAcknowledgeFileRequest {
    public_key: String,
    filename: String,
}
impl WsMsgData for WsMsgAcknowledgeFileRequest {}

#[derive(Serialize)]
struct WsMsgPrepareForFileTransfer {
    public_key: String,
    filename: String,
    amount_of_chunks: u32,
}
impl WsMsgData for WsMsgPrepareForFileTransfer {}

#[derive(Serialize)]
struct WsMsgSendNextChunk {
    chunk_nr: u32,
}
impl WsMsgData for WsMsgSendNextChunk {}

#[derive(Serialize)]
struct WsMsgAddChunk {
    is_last_chunk: bool,
    chunk_nr: u32,
    chunk: String,
    iv: String,
}
impl WsMsgData for WsMsgAddChunk {}

async fn start_listeners(
    tx: mpsc::Sender<String>,
    mut shutdown_rx: watch::Receiver<()>,
    rcm: State<ConnectionManager>,
    session_id: &String,
    user_id: &String,
) -> Result<(), (StatusCode, String)> {
    info!("Start listening.");
    if LISTENERS.insert(user_id.clone(), ()).is_none() {
        let session_id = session_id.clone();
        let user_id = user_id.clone();

        let mut interval = time::interval(Duration::from_millis(100));
        // let mut interval = time::interval(Duration::from_secs(5));

        tokio::spawn(async move {
            loop {
                tokio::select! {
                        _ = interval.tick() => {
                        // acknowledge-file-request \\
                        match msg_acknowledge_file_request(tx.clone(), rcm.clone(), &session_id, &user_id)
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Message acknowledge-file-request failed: {}", e);
                                continue;
                            }
                        }

                        // prepare-for-file-transfer \\
                        match msg_prepare_for_file_request(tx.clone(), rcm.clone(), &user_id)
                            .await
                        {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Message prepare-for-file-transfer failed: {}", e);
                                continue;
                            }
                        }

                        // send-next-chunk \\
                        match msg_send_next_chunk(tx.clone(), rcm.clone(), &user_id).await {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Message send-next-chunk failed: {}", e);
                                continue;
                            }
                        }

                        // add-chunk \\
                        match msg_add_chunk(tx.clone(), rcm.clone(), &user_id).await {
                            Ok(_) => (),
                            Err(e) => {
                                error!("Message add-chunk failed: {}", e);
                                continue;
                            }
                        }
                    }
                    _ = shutdown_rx.changed() => {
                        info!("Listener shutdown signal received.");
                        LISTENERS.remove(&user_id);
                        break;
                    }
                }
            }

            info!("Listener terminated.");
        });
    } else {
        return Err((
            StatusCode::CONFLICT,
            "Listener already running.".to_string(),
        ));
    }

    Ok(())
}

#[derive(Deserialize)]
struct ReqRequestFile {
    public_key: String,
    filename: String,
}

async fn request_file(
    rcm: State<ConnectionManager>,
    session_id: &String,
    user_id: &String,
    data: &String,
) -> Result<(), (StatusCode, String)> {
    info!("request_file");
    let data = utils::deserialize_data::<ReqRequestFile>(&data)?;

    let key = format!("files:{}", &session_id);
    if !utils::redis_handler::sismember(rcm.clone(), &key, &data.filename).await? {
        return Err((StatusCode::NOT_FOUND, "File not found.".to_string()));
    }

    let key = format!("files:{}:{}", &session_id, &data.filename);
    if &utils::redis_handler::hget(rcm.clone(), &key, "owner.id").await? == user_id {
        return Err((
            StatusCode::BAD_REQUEST,
            "You cannot request your own file.".to_string(),
        ));
    }

    let key = format!("file.reqs.receiver:{}", &user_id);
    let request_ids = match utils::redis_handler::smembers(rcm.clone(), &key).await {
        Ok(request_ids) => request_ids,
        Err(_) => Vec::new(),
    };

    if request_ids.len() > 0 {
        return Err((
            StatusCode::CONFLICT,
            "You have already requested this file.".to_string(),
        ));
    }

    let key = format!("file.reqs:{}", &session_id);
    utils::redis_handler::sadd(rcm.clone(), &key, &data.filename, None).await?;

    let key = format!("file.reqs:{}:{}", &session_id, &data.filename);
    utils::redis_handler::sadd(rcm.clone(), &key, &user_id, None).await?;

    let key = format!("file.req:{}:{}:{}", &session_id, &data.filename, &user_id);
    utils::redis_handler::set(rcm.clone(), &key, &data.public_key, None).await?;

    utils::prolong_session(rcm, &session_id).await;

    Ok(())
}

#[derive(Deserialize)]
struct ReqAcknowledgeFileRequest {
    request_id: String,
    public_key: String,
    amount_of_chunks: u32,
    filename: String,
}

async fn acknowledge_file_request(
    rcm: State<ConnectionManager>,
    data: &String,
) -> Result<(), (StatusCode, String)> {
    info!("acknowledge_file_request");

    let data = utils::deserialize_data::<ReqAcknowledgeFileRequest>(&data)?;

    let items = [
        ("filename", data.filename.as_str()),
        ("public.key", data.public_key.as_str()),
        ("amount.of.chunks", &data.amount_of_chunks.to_string()),
    ];

    let key = format!("file.req.prep:{}", &data.request_id);
    utils::redis_handler::hset_multiple(rcm.clone(), &key, &items, None).await?;

    Ok(())
}

#[derive(Deserialize)]
struct ReqReadyForFileRequest {
    request_id: String,
}

async fn ready_for_file_transfer(
    rcm: State<ConnectionManager>,
    user_id: &String,
    data: &String,
) -> Result<(), (StatusCode, String)> {
    info!("ready_for_file_transfer");

    let data = utils::deserialize_data::<ReqReadyForFileRequest>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    let key = format!("chunk.curr:{}", &data.request_id);
    utils::redis_handler::set(rcm, &key, "1", None).await?;

    Ok(())
}

#[derive(Deserialize)]
struct ReqAddChunk {
    request_id: String,
    is_last_chunk: bool,
    chunk_nr: u32,
    chunk: String,
    iv: String,
}

async fn add_chunk(
    rcm: State<ConnectionManager>,
    user_id: &String,
    data: &String,
) -> Result<(), (StatusCode, String)> {
    info!("add_chunk");

    let data = utils::deserialize_data::<ReqAddChunk>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    if data.chunk.len() > MAX_CHUNK_SIZE {
        error!("chunk size: {}", data.chunk.len());
        return Err((StatusCode::BAD_REQUEST, "Chunk too big.".to_string()));
    }

    let key = format!("chunk.req:{}", &data.request_id);
    let requested_chunk_nr = utils::redis_handler::get(rcm.clone(), &key).await?;

    if data.chunk_nr != requested_chunk_nr.parse().unwrap_or(0) {
        return Err((StatusCode::BAD_REQUEST, "Wrong chunk number.".to_string()));
    }

    let key = format!("chunk:{}", &data.request_id);
    let chunk_data = format!("{}@{}@{}", &data.chunk_nr, &data.iv, &data.chunk);
    utils::redis_handler::set(rcm.clone(), &key, &chunk_data, None).await?;

    if data.is_last_chunk {
        let key = format!("chunk.is.last:{}", &data.request_id);
        utils::redis_handler::set(rcm.clone(), &key, "true", None).await?;
    }

    Ok(())
}

#[derive(Deserialize)]
struct ReqReceivedChunk {
    request_id: String,
    chunk_nr: u32,
}

async fn received_chunk(
    rcm: State<ConnectionManager>,
    user_id: &String,
    data: &String,
) -> Result<(), (StatusCode, String)> {
    info!("received_chunk");

    let data = utils::deserialize_data::<ReqReceivedChunk>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    let key = format!("chunk.sent:{}", &data.request_id);
    let send_chunk_nr = match utils::redis_handler::get(rcm.clone(), &key).await {
        Ok(chunk_nr) => chunk_nr.parse().unwrap_or(0),
        Err(_) => return Err((StatusCode::NOT_FOUND, "Chunk not found.".to_string())),
    };

    if data.chunk_nr != send_chunk_nr {
        return Err((StatusCode::CONFLICT, "Chunk number mismatch.".to_string()));
    }

    let key = format!("chunk.is.last:{}", &data.request_id);
    let was_last_chunk = match utils::redis_handler::get(rcm.clone(), &key).await {
        Ok(is_last_chunk) => is_last_chunk == "true",
        Err(_) => false,
    };

    if was_last_chunk {
        utils::redis_handler::del(rcm.clone(), &key).await?;

        let key = format!("chunk.curr:{}", &data.request_id);
        utils::redis_handler::del(rcm.clone(), &key).await?;

        let key = format!("file.req.users:{}", &data.request_id);
        let users = match utils::redis_handler::smembers(rcm.clone(), &key).await {
            Ok(users) => users,
            Err(_) => Vec::new(),
        };
        utils::redis_handler::del(rcm.clone(), &key).await?;

        for user_id in users {
            let key = format!("file.reqs.sender:{}", &user_id);
            match utils::redis_handler::del(rcm.clone(), &key).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to delete file.reqs.sender:user.id");
                }
            };

            let key = format!("file.reqs.receiver:{}", &user_id);
            match utils::redis_handler::del(rcm.clone(), &key).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to delete file.reqs.receiver:user.id");
                }
            };
        }
    } else {
        let key = format!("chunk.curr:{}", &data.request_id);
        utils::redis_handler::incr(rcm.clone(), &key, None).await?;
    }

    let key = format!("chunk.sent:{}", &data.request_id);
    utils::redis_handler::del(rcm.clone(), &key).await?;

    let key = format!("chunk:{}", &data.request_id);
    utils::redis_handler::del(rcm.clone(), &key).await?;

    let key = format!("chunk.req:{}", &data.request_id);
    utils::redis_handler::del(rcm.clone(), &key).await?;

    Ok(())
}

async fn msg_acknowledge_file_request(
    tx: mpsc::Sender<String>,
    rcm: State<ConnectionManager>,
    session_id: &String,
    user_id: &String,
) -> Result<(), String> {
    let user_files = match utils::get_user_files(rcm.clone(), &session_id, &user_id).await {
        Ok(files) => files,
        Err(_) => Vec::new(),
    };
    if user_files.is_empty() {
        return Ok(());
    }

    for file in user_files {
        let key = format!("file.reqs:{}", &session_id);
        match utils::redis_handler::sismember(rcm.clone(), &key, &file).await {
            Ok(_) => (),
            Err(_) => continue,
        };

        match utils::redis_handler::srem(rcm.clone(), &key, &file).await {
            Ok(_) => (),
            Err(_) => {
                error!("Failed to delete file.reqs:session.id");
            }
        }

        let key = format!("file.reqs:{}:{}", &session_id, &file);
        let user_ids = match utils::redis_handler::smembers(rcm.clone(), &key).await {
            Ok(user_ids) => user_ids,
            Err(_) => continue,
        };

        for rec_user_id in user_ids {
            let key = format!("file.req:{}:{}:{}", &session_id, &file, &rec_user_id);
            let public_key = match utils::redis_handler::get(rcm.clone(), &key).await {
                Ok(public_key) => public_key,
                Err(_) => continue,
            };

            match utils::redis_handler::del(rcm.clone(), &key).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to delete file.req:session.id:filename:user.id");
                }
            };

            let key = format!("file.reqs:{}:{}", &session_id, &file);
            match utils::redis_handler::srem(rcm.clone(), &key, &rec_user_id).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to delete file.reqs:session.id:filename");
                }
            }

            let request_id = utils::get_uuid();

            let key = format!("file.req.users:{}", &request_id);
            match utils::redis_handler::sadd(rcm.clone(), &key, &rec_user_id, None).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to add file.req.users:request.id");
                    continue;
                }
            }
            match utils::redis_handler::sadd(rcm.clone(), &key, &user_id, None).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to add file.req.users:request.id");
                    continue;
                }
            }

            let key = format!("file.reqs.receiver:{}", &rec_user_id);
            match utils::redis_handler::sadd(rcm.clone(), &key, &request_id, None).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to add file.reqs.receiver:user.id");
                    continue;
                }
            }
            let key = format!("file.reqs.sender:{}", &user_id);
            match utils::redis_handler::sadd(rcm.clone(), &key, &request_id, None).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to add file.reqs.sender:user.id");
                    continue;
                }
            }

            let message = WsMessage {
                request_id,
                command: "acknowledge-file-request".to_string(),
                data: WsMsgAcknowledgeFileRequest {
                    public_key: public_key.clone(),
                    filename: file.clone(),
                },
            };

            let message_str = serde_json::to_string(&message).unwrap();
            if tx.send(message_str).await.is_err() {
                return Err("Receiver dropped".to_string());
            }
        }
    }

    Ok(())
}

async fn msg_prepare_for_file_request(
    tx: mpsc::Sender<String>,
    rcm: State<ConnectionManager>,
    user_id: &String,
) -> Result<(), String> {
    let key = format!("file.reqs.receiver:{}", &user_id);
    let request_ids = utils::redis_handler::smembers(rcm.clone(), &key)
        .await
        .unwrap_or(Vec::new());

    for request_id in request_ids {
        let key = format!("file.req.prep:{}", &request_id);
        let req_data = match utils::redis_handler::hgetall(rcm.clone(), &key).await {
            Ok(data) => data,
            Err(_) => {
                error!(
                    "Failed to get file request data for request ID: {}",
                    &request_id
                );
                continue;
            }
        };

        if req_data.len() < 6 {
            continue;
        }

        match utils::redis_handler::del(rcm.clone(), &key).await {
            Ok(_) => (),
            Err(_) => {
                error!(
                    "Failed to delete file.req.prep:request_id for request ID: {}",
                    &request_id
                );
            }
        };

        let filename = utils::get_hash_value(&req_data, "filename");
        if filename.is_none() {
            continue;
        }
        let filename = filename.unwrap();

        let public_key = utils::get_hash_value(&req_data, "public.key");
        if public_key.is_none() {
            continue;
        }
        let public_key = public_key.unwrap();

        let amount_of_chunks = utils::get_hash_value(&req_data, "amount.of.chunks");
        if amount_of_chunks.is_none() {
            continue;
        }
        let amount_of_chunks = amount_of_chunks.unwrap();

        let message = WsMessage {
            request_id: request_id.clone(),
            command: "prepare-for-file-transfer".to_string(),
            data: WsMsgPrepareForFileTransfer {
                public_key: public_key.clone(),
                filename: filename.clone(),
                amount_of_chunks: amount_of_chunks.parse().unwrap_or(0),
            },
        };

        let message_str = serde_json::to_string(&message).unwrap();
        if tx.send(message_str).await.is_err() {
            return Err("Receiver dropped".to_string());
        }
    }

    Ok(())
}

async fn msg_send_next_chunk(
    tx: mpsc::Sender<String>,
    rcm: State<ConnectionManager>,
    user_id: &String,
) -> Result<(), String> {
    let key = format!("file.reqs.sender:{}", &user_id);
    let request_ids = match utils::redis_handler::smembers(rcm.clone(), &key).await {
        Ok(request_ids) => request_ids,
        Err(_) => Vec::new(),
    };

    for request_id in request_ids {
        let key = format!("chunk.curr:{}", &request_id);
        let chunk_nr = match utils::redis_handler::get(rcm.clone(), &key).await {
            Ok(chunk) => chunk,
            Err(_) => continue,
        };

        let key = format!("chunk.req:{}", &request_id);
        let requested_chunk_nr = match utils::redis_handler::get(rcm.clone(), &key).await {
            Ok(chunk_nr) => chunk_nr,
            Err(_) => "".to_string(),
        };
        if !requested_chunk_nr.is_empty() {
            continue;
        }

        match utils::redis_handler::set(rcm.clone(), &key, &chunk_nr, None).await {
            Ok(_) => (),
            Err(_) => {
                error!("Failed to set chunk.req:request.id");
                continue;
            }
        };

        let message = WsMessage {
            request_id: request_id.clone(),
            command: "send-next-chunk".to_string(),
            data: WsMsgSendNextChunk {
                chunk_nr: chunk_nr.parse().unwrap_or(0),
            },
        };

        let message_str = serde_json::to_string(&message).unwrap();
        if tx.send(message_str).await.is_err() {
            return Err("Receiver dropped".to_string());
        }
    }

    Ok(())
}

async fn msg_add_chunk(
    tx: mpsc::Sender<String>,
    rcm: State<ConnectionManager>,
    user_id: &String,
) -> Result<(), String> {
    let key = format!("file.reqs.receiver:{}", &user_id);
    let request_ids = match utils::redis_handler::smembers(rcm.clone(), &key).await {
        Ok(request_ids) => request_ids,
        Err(_) => Vec::new(),
    };

    for request_id in request_ids {
        let key = format!("chunk.sent:{}", &request_id);
        let sent_chunk_nr = match utils::redis_handler::get(rcm.clone(), &key).await {
            Ok(chunk_nr) => chunk_nr,
            Err(_) => "".to_string(),
        };
        if !sent_chunk_nr.is_empty() {
            continue;
        }

        let key = format!("chunk:{}", &request_id);
        let chunk_data = match utils::redis_handler::get(rcm.clone(), &key).await {
            Ok(chunk_data) => chunk_data,
            Err(_) => continue,
        };

        if chunk_data.is_empty() {
            continue;
        }

        let chunk_split = chunk_data.split('@').collect::<Vec<&str>>();
        if chunk_split.len() != 3 {
            error!("Invalid chunk data: {}", &chunk_data);
            continue;
        }

        let chunk_nr = chunk_split[0].parse().unwrap_or(0);
        let iv = chunk_split[1].to_string();
        let chunk = chunk_split[2].to_string();

        let key = format!("chunk.curr:{}", &request_id);
        let curr_chunk_nr = match utils::redis_handler::get(rcm.clone(), &key).await {
            Ok(curr_chunk_nr) => curr_chunk_nr,
            Err(_) => continue,
        };
        if chunk_nr != curr_chunk_nr.parse().unwrap_or(0) {
            continue;
        }

        let key = format!("chunk.sent:{}", &request_id);
        match utils::redis_handler::set(rcm.clone(), &key, &chunk_nr.to_string(), None).await {
            Ok(_) => (),
            Err(_) => {
                error!("Failed to set chunk.sent:request.id");
                continue;
            }
        };

        let key = format!("chunk.is.last:{}", &request_id);
        let is_last_chunk = match utils::redis_handler::get(rcm.clone(), &key).await {
            Ok(is_last_chunk) => is_last_chunk.parse().unwrap_or(false),
            Err(_) => false,
        };

        let message = WsMessage {
            request_id: request_id.clone(),
            command: "add-chunk".to_string(),
            data: WsMsgAddChunk {
                is_last_chunk,
                chunk_nr,
                chunk,
                iv,
            },
        };

        let message_str = serde_json::to_string(&message).unwrap();
        if tx.send(message_str).await.is_err() {
            return Err("Receiver dropped".to_string());
        }
    }

    Ok(())
}

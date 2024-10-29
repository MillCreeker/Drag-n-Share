#[allow(unused_imports)] // TODO
use axum::routing::head;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};

use tokio::net::TcpListener;
use utils::handle_redis_error;

use once_cell::sync::Lazy;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::sync::{mpsc, watch};
use tokio::time;
use tokio_stream::StreamExt;

use dashmap::DashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use axum_client_ip::{SecureClientIp, SecureClientIpSource};

use http::header::HeaderValue;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use redis::aio::ConnectionManager;

use env_logger::Env;
use log::{error, info};

const MAX_CHUNK_SIZE: usize = 2400;
const MAX_QUEUE_SIZE: i64 = 16;

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
        // .allow_origin(AllowOrigin::exact(
        //     HeaderValue::from_str("https://drag-n-share.com").unwrap(),
        // )) // TODO
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

#[derive(Serialize)]
struct Response {
    success: bool,
    response: String,
}

async fn ws_handler(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    Path(session_id): Path<String>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    info!("Websocket connection opened");
    ws.on_upgrade(|ws| ws_handler_inner(rcm, secure_ip, session_id, ws))
}

async fn ws_handler_inner(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
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
                        info!("Received message from client: {}", text);

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
                Ok(Response {
                    success: false,
                    response: "No user ID".to_string(),
                })
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
                acknowledge_file_request(
                    rcm.clone(),
                    session_id,
                    &user_id.unwrap_or("".to_string()),
                    &request.data,
                )
                .await
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
            } else {
                Ok(Response {
                    success: false,
                    response: format!("Unknown command: {}", request.command),
                })
            }
            .unwrap_or(Response {
                success: false,
                response: "Error".to_string(),
            });

            info!("Response: {:?}", response.response);
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
    user_id: String,
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
    last_chunk_nr: u32,
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
) -> Result<Response, String> {
    info!("start listening");
    info!("{:?}", LISTENERS);
    if LISTENERS.insert(user_id.clone(), ()).is_none() {
        let session_id = session_id.clone();
        let user_id = user_id.clone();

        let mut interval = time::interval(Duration::from_millis(500));
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
                        match msg_prepare_for_file_request(tx.clone(), rcm.clone(), &session_id, &user_id)
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
        return Err("Listener already started".to_string());
    }

    Ok(Response {
        success: true,
        response: "Registered".to_string(),
    })
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
) -> Result<Response, String> {
    info!("Requesting file");
    let data = utils::deserialize_data::<ReqRequestFile>(&data)?;

    let key = format!("files:{}", &session_id);
    if !utils::handle_redis_error(
        utils::redis_handler::sismember(rcm.clone(), &key, &data.filename).await,
    )? {
        return Err("File not found".to_string());
    }

    let key = format!("files:{}:{}", &session_id, &data.filename);
    if &handle_redis_error(utils::redis_handler::hget(rcm.clone(), &key, "owner.id").await)?
        == user_id
    {
        return Err("You cannot request your own file".to_string());
    }

    let key = format!("file.reqs:{}", &session_id);
    utils::handle_redis_error(
        utils::redis_handler::sadd(rcm.clone(), &key, &data.filename, None).await,
    )?;

    let key = format!("file.reqs:{}:{}", &session_id, &data.filename);
    utils::handle_redis_error(utils::redis_handler::sadd(rcm.clone(), &key, &user_id, None).await)?;

    let key = format!("file.req:{}:{}:{}", &session_id, &data.filename, &user_id);
    utils::handle_redis_error(utils::redis_handler::set(rcm, &key, &data.public_key, None).await)?;

    Ok(Response {
        success: true,
        response: "".to_string(),
    })
}

#[derive(Deserialize)]
struct ReqAcknowledgeFileRequest {
    request_id: String,
    public_key: String,
    amount_of_chunks: u32,
    filename: String,
    user_id: String,
}

async fn acknowledge_file_request(
    rcm: State<ConnectionManager>,
    session_id: &String,
    user_id: &String,
    data: &String,
) -> Result<Response, String> {
    let data = utils::deserialize_data::<ReqAcknowledgeFileRequest>(&data)?;

    let key = format!(
        "file.req.ackn:{}:{}:{}",
        &session_id, &data.filename, &data.user_id
    );
    utils::handle_redis_error(
        utils::redis_handler::set(rcm.clone(), &key, &data.request_id, None).await,
    )?;

    let items = [
        ("filename", data.filename.as_str()),
        ("public.key", data.public_key.as_str()),
        ("amount.of.chunks", &data.amount_of_chunks.to_string()),
    ];

    let key = format!("file.req.prep:{}", &data.request_id);
    utils::handle_redis_error(
        utils::redis_handler::hset_multiple(rcm.clone(), &key, &items, None).await,
    )?;

    Ok(Response {
        success: true,
        response: "".to_string(),
    })
}

#[derive(Deserialize)]
struct ReqReadyForFileRequest {
    request_id: String,
}

async fn ready_for_file_transfer(
    rcm: State<ConnectionManager>,
    user_id: &String,
    data: &String,
) -> Result<Response, String> {
    let data = utils::deserialize_data::<ReqReadyForFileRequest>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    let key = format!("file.req.ready:{}", &data.request_id);
    utils::handle_redis_error(utils::redis_handler::set(rcm.clone(), &key, "true", None).await)?;

    Ok(Response {
        success: true,
        response: "".to_string(),
    })
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
) -> Result<Response, String> {
    info!("add_chunk");

    let data = utils::deserialize_data::<ReqAddChunk>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    if data.chunk.len() > MAX_CHUNK_SIZE {
        error!("Chunk is too large: {}", &data.chunk.len());
        return Err("Chunk is too large".to_string());
    }

    let key = format!("file.req.chunks:{}", &data.request_id);
    let queue_size =
        utils::handle_redis_error(utils::redis_handler::llen(rcm.clone(), &key).await)?;

    if queue_size >= MAX_QUEUE_SIZE {
        return Err("Queue is full".to_string());
    }

    let queue_data = format!("{}@{}@{}", &data.chunk_nr, &data.iv, &data.chunk);
    utils::handle_redis_error(utils::redis_handler::lpush(rcm.clone(), &key, &queue_data, None).await)?;

    if data.is_last_chunk {
        utils::handle_redis_error(utils::redis_handler::lpush(rcm.clone(), &key, "FIN", None).await)?;

        let key = format!("file.req.last.chunk:{}", &data.request_id);
        utils::handle_redis_error(
            utils::redis_handler::set(rcm.clone(), &key, "FIN", None).await,
        )?;
    }

    let key = format!("file.req.last.chunk:{}", &data.request_id);
    utils::handle_redis_error(
        utils::redis_handler::set(rcm, &key, &data.chunk_nr.to_string(), None).await,
    )?;

    Ok(Response {
        success: true,
        response: "".to_string(),
    })
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

    // info!("{:?}", user_files);

    if user_files.is_empty() {
        return Ok(());
    }

    for file in user_files {
        let key = format!("file.reqs:{}", &session_id);
        match utils::redis_handler::sismember(rcm.clone(), &key, &file).await {
            Ok(_) => (),
            Err(_) => continue,
        };

        // TODO del file.reqs:session.id
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

            // TODO del file.reqs:session.id:filename:user.id
            match utils::redis_handler::del(rcm.clone(), &key).await {
                Ok(_) => (),
                Err(_) => {
                    error!("Failed to delete file.req:session.id:filename:user.id");
                }
            };

            // TODO srem file.reqs:session.id:filename
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
                    user_id: user_id.clone(),
                },
            };

            info!("Sending message");
            let message_str = serde_json::to_string(&message).unwrap();
            if tx.send(message_str).await.is_err() {
                error!("Receiver dropped");
                return Err("Receiver dropped".to_string());
            }
            info!("Sending message done");
        }
    }

    Ok(())
}

async fn msg_prepare_for_file_request(
    tx: mpsc::Sender<String>,
    rcm: State<ConnectionManager>,
    session_id: &String,
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

        // TODO del file.req.prep:request_id
        match utils::redis_handler::del(rcm.clone(), &key).await {
            Ok(_) => (),
            Err(_) => {
                error!(
                    "Failed to delete file.req.prep:request_id for request ID: {}",
                    &request_id
                );
            }
        };

        // TODO del file.req.ackn:{session.id}:{filename}:{user.id}
        info!("{:?}", req_data);
        let filename = utils::get_hash_value(&req_data, "filename");
        if filename.is_none() {
            continue;
        }
        let filename = filename.unwrap();
        info!("{}", filename);

        let key = format!("file.req.ackn:{}:{}:{}", &session_id, &filename, &user_id);
        match utils::redis_handler::del(rcm.clone(), &key).await {
            Ok(_) => (),
            Err(_) => {
                error!(
                    "Failed to delete file.req.ackn:session.id:filename:user.id for request ID: {}",
                    &request_id
                );
            }
        }

        let public_key = utils::get_hash_value(&req_data, "public.key");
        if public_key.is_none() {
            continue;
        }
        let public_key = public_key.unwrap();
        info!("{}", public_key);

        let amount_of_chunks = utils::get_hash_value(&req_data, "amount.of.chunks");
        if amount_of_chunks.is_none() {
            continue;
        }
        let amount_of_chunks = amount_of_chunks.unwrap();
        info!("{}", amount_of_chunks);

        let message = WsMessage {
            request_id: request_id.clone(),
            command: "prepare-for-file-transfer".to_string(),
            data: WsMsgPrepareForFileTransfer {
                public_key: public_key.clone(),
                filename: filename.clone(),
                amount_of_chunks: amount_of_chunks.parse().unwrap_or(0),
            },
        };

        info!("Sending message");
        let message_str = serde_json::to_string(&message).unwrap();
        if tx.send(message_str).await.is_err() {
            error!("Receiver dropped");
            return Err("Receiver dropped".to_string());
        }
        info!("Sending message done");
    }

    Ok(())
}

async fn msg_send_next_chunk(
    tx: mpsc::Sender<String>,
    rcm: State<ConnectionManager>,
    user_id: &String,
) -> Result<(), String> {
    let key = format!("file.reqs.sender:{}", &user_id);
    let request_ids = utils::redis_handler::smembers(rcm.clone(), &key)
        .await
        .unwrap_or(Vec::new());

    for request_id in request_ids {
        if !utils::is_request_ready(rcm.clone(), &request_id).await {
            continue;
        }

        let key = format!("file.req.chunks:{}", &request_id);

        let queue_size = match utils::redis_handler::llen(rcm.clone(), &key).await {
            Ok(size) => size,
            Err(_) => {
                error!("Failed to get queue size for request ID: {}", &request_id);
                continue;
            }
        };
        if queue_size >= MAX_QUEUE_SIZE {
            continue;
        }

        let key = format!("file.req.last.chunk:{}", &request_id);
        let last_chunk_nr = match utils::redis_handler::get(rcm.clone(), &key).await {
            Ok(chunk_nr) => chunk_nr,
            Err(_) => {
                let message = WsMessage {
                    request_id: request_id.clone(),
                    command: "send-next-chunk".to_string(),
                    data: WsMsgSendNextChunk { last_chunk_nr: 0 },
                };

                info!("Sending message");
                let message_str = serde_json::to_string(&message).unwrap();
                if tx.send(message_str).await.is_err() {
                    error!("Receiver dropped");
                    return Err("Receiver dropped".to_string());
                }
                info!("Sending message done");
                continue;
            }
        };
        if last_chunk_nr == "FIN" {
            continue;
        }

        let chunk_nr = last_chunk_nr.parse().unwrap_or(0);

        let message = WsMessage {
            request_id: request_id.clone(),
            command: "send-next-chunk".to_string(),
            data: WsMsgSendNextChunk {
                last_chunk_nr: chunk_nr,
            },
        };

        info!("Sending message");
        let message_str = serde_json::to_string(&message).unwrap();
        if tx.send(message_str).await.is_err() {
            error!("Receiver dropped");
            return Err("Receiver dropped".to_string());
        }
        info!("Sending message done");
    }

    Ok(())
}

async fn msg_add_chunk(
    tx: mpsc::Sender<String>,
    rcm: State<ConnectionManager>,
    user_id: &String,
) -> Result<(), String> {
    let key = format!("file.reqs.receiver:{}", &user_id);
    let request_ids = utils::redis_handler::smembers(rcm.clone(), &key)
        .await
        .unwrap_or(Vec::new());

    for request_id in request_ids {
        if !utils::is_request_ready(rcm.clone(), &request_id).await {
            continue;
        }

        let key = format!("file.req.chunks:{}", &request_id);
        let chunk = match utils::redis_handler::rpop(rcm.clone(), &key).await {
            Ok(chunk) => chunk,
            Err(_) => {
                error!("Failed to get next chunk from queue: {}", &request_id);
                continue;
            }
        };

        let mut message = WsMessage {
            request_id: request_id.clone(),
            command: "add-chunk".to_string(),
            data: WsMsgAddChunk {
                is_last_chunk: true,
                chunk_nr: 0,
                chunk: "".to_string(),
                iv: "".to_string(),
            },
        };

        if chunk == "FIN" {
            clean_up_request_data(rcm.clone(), &request_id).await;
            info!("<< Last one >>");
            info!("Sending message");
            let message_str = serde_json::to_string(&message).unwrap();
            info!("{}", message_str);
            if tx.send(message_str).await.is_err() {
                error!("Receiver dropped");
                return Err("Receiver dropped".to_string());
            }
            info!("Sending message done");
        } else {
            let chunk_parts: Vec<&str> = chunk.split('@').collect();
            let chunk_nr = chunk_parts[0].parse().unwrap_or(0);

            message.data = WsMsgAddChunk {
                is_last_chunk: false,
                chunk_nr,
                chunk: chunk_parts[2].to_string(),
                iv: chunk_parts[1].to_string(),
            };

            info!("Sending message");
            let message_str = serde_json::to_string(&message).unwrap();
            if tx.send(message_str).await.is_err() {
                error!("Receiver dropped");
                return Err("Receiver dropped".to_string());
            }
            info!("Sending message done");
        }
    }

    Ok(())
}

async fn clean_up_request_data(rcm: State<ConnectionManager>, request_id: &String) -> bool {
    let mut was_successful = true;

    // TODO del file.req.users:{request.id}
    let key = format!("file.req.users:{}", &request_id);
    let users = match utils::redis_handler::smembers(rcm.clone(), &key).await {
        Ok(users) => users,
        Err(_) => {
            was_successful = false;
            return was_successful;
        }
    };

    match utils::redis_handler::del(rcm.clone(), &key).await {
        Ok(_) => (),
        Err(_) => {
            error!("Failed to delete file.req.users:request.id");
            was_successful = false;
        }
    }

    for user in users {
        // TODO srem file.reqs.sender:{user.id}
        let key = format!("file.reqs.sender:{}", &user);
        match utils::redis_handler::srem(rcm.clone(), &key, &request_id).await {
            Ok(_) => (),
            Err(_) => {
                error!("Failed to delete file.reqs.sender:user.id");
                was_successful = false;
            }
        }

        // TODO srem file.reqs.receiver:{user.id}
        let key = format!("file.reqs.receiver:{}", &user);
        match utils::redis_handler::srem(rcm.clone(), &key, &request_id).await {
            Ok(_) => (),
            Err(_) => {
                error!("Failed to delete file.reqs.receiver:user.id");
                was_successful = false;
            }
        }

        // TODO del file.req.ready:{request.id}
        let key = format!("file.req.ready:{}", &request_id);
        match utils::redis_handler::del(rcm.clone(), &key).await {
            Ok(_) => (),
            Err(_) => {
                error!("Failed to delete file.req.ready:request.id");
                was_successful = false;
            }
        }

        // TODO del file.req.chunks:{request.id}
        let key = format!("file.req.chunks:{}", &request_id);
        match utils::redis_handler::del(rcm.clone(), &key).await {
            Ok(_) => (),
            Err(_) => {
                error!("Failed to delete file.req.chunks:request.id");
                was_successful = false;
            }
        }
    }

    was_successful
}

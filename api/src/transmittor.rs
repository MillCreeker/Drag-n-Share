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
use utils::{handle_redis_error, redis_handler::sadd};

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{watch, Mutex};
use tokio::time;
use tokio_stream::StreamExt;

use serde::{Deserialize, Serialize};

use axum_client_ip::{SecureClientIp, SecureClientIpSource};

use http::header::HeaderValue;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};

use redis::{aio::ConnectionManager, AsyncCommands};

use std::net::SocketAddr;

use env_logger::Env;
use log::{error, info};

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

#[allow(unused_variables)] // TODO
async fn ws_handler_inner(
    rcm: State<ConnectionManager>,
    secure_ip: SecureClientIp,
    session_id: String,
    ws: WebSocket,
) {
    let ws = Arc::new(Mutex::new(ws));

    // create a watch channel to signal shutdown
    let (shutdown_tx, shutdown_rx) = watch::channel(());

    // spawn the Redis listeners, passing the shutdown receiver to each one
    let _listener1 = tokio::spawn(redis_listener(ws.clone(), rcm.clone(), shutdown_rx.clone()));
    // let _listener2 = tokio::spawn(redis_listener(
    //     ws.clone(),
    //     rcm.clone(),
    //     shutdown_rx.clone(),
    // ));

    // listen for incoming messages and process them
    while let Some(Ok(message)) = ws.lock().await.next().await {
        if let Message::Text(text) = message {
            info!("Received message: {}", text);
            // try to deserialize the incoming message
            match serde_json::from_str::<Request>(&text) {
                Ok(request) => {
                    let user_id: Option<String> = match utils::decode_jwt(&request.jwt) {
                        Ok(claims) => {
                            if claims.aud != session_id {
                                error!("Invalid session ID");
                                None
                            } else {
                                Some(claims.sub)
                            }
                        }
                        Err(e) => {
                            error!("Failed to decode JWT: {}", request.jwt);
                            None
                        }
                    };

                    //////////////////////////////////////////////////
                    // handle commands \\
                    let response = if user_id.is_none() {
                        Ok(Response {
                            success: false,
                            response: "No user ID".to_string(),
                        })
                    } else if request.command == "request-file" {
                        request_file(
                            rcm.clone(),
                            &session_id,
                            &user_id.unwrap_or("".to_string()),
                            &request.data,
                        )
                        .await
                    } else if request.command == "acknowledge-file-request" {
                        acknowledge_file_request(
                            rcm.clone(),
                            &session_id,
                            &user_id.unwrap_or("".to_string()),
                            &request.data,
                        )
                        .await
                    } else if request.command == "ready-for-file-transfer" {
                        ready_for_file_transfer(
                            rcm.clone(),
                            &session_id,
                            &user_id.unwrap_or("".to_string()),
                            &request.data,
                        )
                        .await
                    } else if request.command == "add-chunk" {
                        add_chunk(
                            rcm.clone(),
                            &session_id,
                            &user_id.unwrap_or("".to_string()),
                            &request.data,
                        )
                        .await
                    } else if request.command == "received-chunk" {
                        received_chunk(
                            rcm.clone(),
                            &session_id,
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
                    //////////////////////////////////////////////////

                    // serialize the response to JSON
                    match serde_json::to_string(&response) {
                        Ok(json) => {
                            // Send the response back as a text message
                            if let Err(e) = ws.lock().await.send(Message::Text(json)).await {
                                error!("Failed to send message: {}", e);
                                return;
                            }
                        }
                        Err(e) => error!("Failed to serialize outgoing message: {}", e),
                    }
                }
                Err(e) => error!("Failed to deserialize incoming message: {}", e),
            }
        }
    }

    // WebSocket connection has been closed, notify listeners to shut down
    let _ = shutdown_tx.send(());
    info!("WebSocket closed, sent shutdown signal to listeners.");
}

#[allow(unused_variables)] // TODO
async fn redis_listener(
    ws: Arc<Mutex<WebSocket>>,
    rcm: State<ConnectionManager>,
    mut shutdown_signal: watch::Receiver<()>,
) {
    let mut interval = time::interval(Duration::from_secs(1)); // poll every second

    loop {
        tokio::select! {
            _ = interval.tick() => {
                // rcm.sismember(key, member)
                // Check Redis for a specific key/value update
                // match redis_connection.clone().get::<String, Option<String>>(channel.clone()).await {
                //     Ok(Some(value)) => {
                //         // Send the value found in Redis to the WebSocket
                //         let response = Response {
                //             success: true,
                //             response: format!("Found update in Redis: {}", value),
                //         };
                //         let json = serde_json::to_string(&response).unwrap_or_else(|_| {
                //             "{\"success\":false,\"response\":\"Serialization error\"}".to_string()
                //         });

                //         if let Err(e) = ws.send(Message::Text(json)).await {
                //             error!("Failed to send message: {}", e);
                //             return;
                //         }
                //     }
                //     Ok(None) => {
                //         // No update found; you can log or perform other actions here
                //     }
                //     Err(e) => {
                //         error!("Failed to query Redis: {}", e);
                //     }
                // }
                info!("Tick");
            }

            _ = shutdown_signal.changed() => {
                break;
            }
        }
    }

    info!("Listener terminated.");
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

    let request_id = utils::get_uuid();

    let key = format!(
        "file.req.ackn:{}:{}:{}",
        &session_id, &data.filename, &data.user_id
    );
    utils::handle_redis_error(
        utils::redis_handler::set(rcm.clone(), &key, &request_id, None).await,
    )?;

    let key = format!("file.req.users:{}", &request_id);
    utils::handle_redis_error(
        utils::redis_handler::sadd(rcm.clone(), &key, &data.user_id, None).await,
    )?;
    utils::handle_redis_error(utils::redis_handler::sadd(rcm.clone(), &key, &user_id, None).await)?;

    let items = [
        ("public.key", data.public_key.as_str()),
        ("amount.of.chunks", &data.amount_of_chunks.to_string()),
    ];

    let key = format!("file.req.prep:{}", &request_id);
    utils::handle_redis_error(
        utils::redis_handler::hset_multiple(rcm.clone(), &key, &items, None).await,
    )?;

    let key = format!(
        "file.req:{}:{}:{}",
        &session_id, &data.filename, &data.user_id
    );
    utils::handle_redis_error(utils::redis_handler::del(rcm, &key).await)?;

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
    session_id: &String,
    user_id: &String,
    data: &String,
) -> Result<Response, String> {
    let data = utils::deserialize_data::<ReqReadyForFileRequest>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    let key = format!("file.req.ready:{}", &data.request_id);
    utils::handle_redis_error(utils::redis_handler::set(rcm.clone(), &key, "true", None).await)?;

    let key = format!("file.req.prep:{}", &data.request_id);
    utils::handle_redis_error(utils::redis_handler::del(rcm.clone(), &key).await)?;

    Ok(Response {
        success: true,
        response: "".to_string(),
    })
}

#[derive(Deserialize)]
struct ReqAddChunk {
    request_id: String,
    chunk_nr: u32,
    chunk: String,
    iv: String,
}

async fn add_chunk(
    rcm: State<ConnectionManager>,
    session_id: &String,
    user_id: &String,
    data: &String,
) -> Result<Response, String> {
    let data = utils::deserialize_data::<ReqAddChunk>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    // TODO
    // lpush, rpop, rpush

    Ok(Response {
        success: true,
        response: "".to_string(),
    })
}

#[derive(Deserialize)]
struct ReqReceivedChunk {
    request_id: String,
    chunk_nr: u32,
}

async fn received_chunk(
    rcm: State<ConnectionManager>,
    session_id: &String,
    user_id: &String,
    data: &String,
) -> Result<Response, String> {
    let data = utils::deserialize_data::<ReqReceivedChunk>(&data)?;

    utils::check_user_is_in_file_request(rcm.clone(), &data.request_id, user_id).await?;

    // TODO

    Ok(Response {
        success: true,
        response: "".to_string(),
    })
}

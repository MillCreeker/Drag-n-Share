use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
};
use axum_client_ip::SecureClientIp;
use rand::seq::SliceRandom;
use redis::aio::ConnectionManager;

use serde_json::json;

use crate::redis_handler;

use rand::Rng;
use sha256::digest;

use uuid::Uuid;

pub async fn get_redis_connection_manager(
) -> Result<redis::aio::ConnectionManager, redis::RedisError> {
    let database_password = std::env::var("DATABASE_PASSWORD").unwrap_or_default();

    let redis_conn_url = format!("redis://:{}@database:6379/", database_password);
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

const CALL_RATE_LIMIT_SEC: i64 = 1;

pub async fn handle_call_rate_limit(
    rcm: State<ConnectionManager>,
    ref ip: &SecureClientIp,
) -> Result<(), (StatusCode, String)> {
    let ip = ip.0.to_string();

    if redis_handler::sismember(rcm.clone(), "calls", &ip).await? {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            json!({
                "success": false,
                "message": "rate limit exceeded"
            })
            .to_string(),
        ));
    }

    redis_handler::sadd(rcm.clone(), "calls", &ip, Some(CALL_RATE_LIMIT_SEC)).await?;

    Ok(())
}

pub async fn check_session_exists(
    rcm: State<ConnectionManager>,
    ref session_name: &str,
) -> Result<(), (StatusCode, String)> {
    let key = format!("session:{}", session_name);

    if !redis_handler::exists(rcm.clone(), &key).await? {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "session name not found"
            })
            .to_string(),
        ));
    }

    Ok(())
}

pub async fn check_is_user_authorized(
    rcm: State<ConnectionManager>,
    ref headers: &HeaderMap,
    ref session_name: &str,
) -> Result<(), (StatusCode, String)> {
    let auth = get_header(&headers, "authorization")?;

    // TODO auslagern
    let auth_parts = auth.split(" ");
    let token = auth_parts.last().unwrap(); // without "Bearer "
    let encrypted_token = sha256(token);

    let user = get_header(&headers, "user-agent")?;

    let token_key = format!("session:{}:{}", session_name, user);

    let actual_token = redis_handler::get(rcm, &token_key).await?;
    if actual_token != encrypted_token {
        return Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "message": "bearer token invalid"
            })
            .to_string(),
        ));
    }

    Ok(())
}

pub async fn get_random_dragon_name(
    rcm: State<ConnectionManager>,
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
    let dragon_name = dragon_names
        .choose(&mut rand::thread_rng())
        .unwrap_or(&dragon_names[0])
        .to_string();
    let key = format!("session:{}", dragon_name);

    if !redis_handler::exists(rcm.clone(), &key)
        .await
        .unwrap_or(false)
    {
        return Ok(dragon_name);
    }

    // any name from list
    for name in dragon_names {
        let key = format!("session:{}", name);
        if !redis_handler::exists(rcm.clone(), &key)
            .await
            .unwrap_or(false)
        {
            return Ok(name.to_string());
        }
    }

    // first random name with counter
    let mut counter = 1;
    loop {
        let nr_key = format!("{}{}", &key, counter);

        if !redis_handler::exists(rcm.clone(), &nr_key)
            .await
            .unwrap_or(false)
        {
            return Ok(format!("{}{}", &key, counter));
        }

        counter += 1;
    }
}

pub fn get_header(ref headers: &HeaderMap, key: &str) -> Result<String, (StatusCode, String)> {
    let header = headers.get("Authorization");
    if header.is_none() {
        return Err((
            StatusCode::BAD_REQUEST,
            json!({
                "success": false,
                "message": format!("{} header not found", key)
            })
            .to_string(),
        ));
    }

    let header = header.unwrap().to_str().unwrap();
    Ok(header.to_string())
}

pub fn get_random_six_digit_code() -> String {
    let code = rand::thread_rng().gen_range(1..999999);
    let code_str = format!("{:06}", code);

    code_str
}

pub fn get_random_access_token() -> String {
    let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890"
        .chars()
        .collect::<Vec<char>>();
    let random_string: String = (0..255)
        .map(|_| chars[rand::thread_rng().gen_range(0..chars.len())])
        .collect();

    random_string
}

pub fn sha256(s: &str) -> String {
    digest(s)
}

pub fn get_uuid() -> String {
    Uuid::new_v4().to_string()
}

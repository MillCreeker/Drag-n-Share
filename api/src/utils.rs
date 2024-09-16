use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
};
use axum_client_ip::SecureClientIp;
use rand::seq::SliceRandom;
use redis::aio::ConnectionManager;

use log::error;

use serde::{Deserialize, Serialize};
use serde_json::json;

use std::time::{SystemTime, UNIX_EPOCH};

use crate::redis_handler;

use rand::Rng;
use sha256::digest;

use uuid::Uuid;

use jsonwebtoken::{self, EncodingKey};

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
    ref session_id: &str,
) -> Result<(), (StatusCode, String)> {
    let key = format!("session:{}", session_id);

    if !redis_handler::exists(rcm.clone(), &key).await? {
        return Err((
            StatusCode::NOT_FOUND,
            json!({
                "success": false,
                "message": "session id not found"
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

    if !redis_handler::exists(rcm.clone(), &key).await? {
        return Ok(dragon_name);
    }

    // any name from list
    for name in dragon_names {
        let key = format!("session:{}", name);
        if !redis_handler::exists(rcm.clone(), &key).await? {
            return Ok(name.to_string());
        }
    }

    // first random name with counter
    let mut counter = 1;
    loop {
        let nr_key = format!("session:{}{}", &dragon_name, counter);

        if !redis_handler::exists(rcm.clone(), &nr_key).await? {
            return Ok(format!("{}{}", &dragon_name, counter));
        }

        counter += 1;
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    aud: String,
    sub: String,
    iat: u128,
    exp: u128,
    is_host: bool,
}

pub fn create_jwt(
    ref session_id: &str,
    ref user_id: Option<&str>,
) -> Result<String, (StatusCode, String)> {
    let jwt_key = std::env::var("JWT_KEY").unwrap_or_default();

    let key = EncodingKey::from_secret(jwt_key.as_ref());
    let header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS256);
    let claims = Claims {
        aud: session_id.to_string(),
        sub: user_id.unwrap_or(&get_uuid()).to_string(),
        iat: get_current_timestamp(),
        exp: get_current_timestamp() + JWT_EXPIRATION_TIME,
        is_host: !user_id.is_none(),
    };

    match jsonwebtoken::encode(&header, &claims, &key) {
        Ok(token) => Ok(token),
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "response": "failed to create jwt"
                })
                .to_string(),
            ))
        }
    }
}

const JWT_EXPIRATION_TIME: u128 = 5 * 60 * 1000; // 5 minutes

pub fn decode_jwt(ref jwt: &str) -> Result<Claims, (StatusCode, String)> {
    let jwt_key = std::env::var("JWT_KEY").map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "response": "failed to locate jwt key"
            })
            .to_string(),
        )
    })?;

    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_aud = false;

    let key = jsonwebtoken::DecodingKey::from_secret(jwt_key.as_ref());

    let decoded_jwt = match jsonwebtoken::decode::<Claims>(&jwt, &key, &validation) {
        Ok(v) => v,
        Err(e) => {
            error!("decode_jwt: {:?}", e);

            return Err((
                StatusCode::UNAUTHORIZED,
                json!({
                    "success": false,
                    "response": "failed to decode jwt"
                })
                .to_string(),
            ))
        }
    };

    let claims = decoded_jwt.claims;
    let now = get_current_timestamp();

    if (now - claims.iat) > JWT_EXPIRATION_TIME {
        return Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "response": "jwt expired"
            })
            .to_string(),
        ));
    }

    Ok(claims)
}

pub fn check_user_is_host(
    ref headers: &HeaderMap,
    session_id: &str,
) -> Result<(), (StatusCode, String)> {
    let auth = get_header(&headers, "authorization")?;
    let parts = auth.split(" ");
    let jwt = parts.last().unwrap_or("");

    let claims = decode_jwt(&jwt)?;

    if claims.aud != session_id {
        return Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "message": "invalid session id"
            })
            .to_string(),
        ));
    }

    if !claims.is_host {
        return Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "message": "permission denied"
            })
            .to_string(),
        ));
    }

    Ok(())
}

pub fn get_current_timestamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub struct User {
    pub id: String,
    pub is_host: bool,
}

pub fn check_user_is_in_session(
    ref headers: &HeaderMap,
    session_id: &str,
) -> Result<User, (StatusCode, String)> {
    let auth = get_header(&headers, "authorization")?;
    let parts = auth.split(" ");
    let jwt = parts.last().unwrap_or("");
    let claims = decode_jwt(&jwt)?;

    if claims.aud != session_id {
        return Err((
            StatusCode::UNAUTHORIZED,
            json!({
                "success": false,
                "message": "invalid session id"
            })
            .to_string(),
        ));
    }

    Ok(User {
        id: claims.sub,
        is_host: claims.is_host,
    })
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

pub fn sha256(s: &str) -> String {
    digest(s)
}

pub fn get_uuid() -> String {
    Uuid::new_v4().to_string()
}

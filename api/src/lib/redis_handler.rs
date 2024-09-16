use axum::{extract::State, http::StatusCode};

use redis::{aio::ConnectionManager, AsyncCommands};

use log::error;
use serde_json::json;

const EXPIRATION_TIME: i64 = 300; // 5min
const DB_ERROR_MSG: &str = "error connection to database";

pub async fn expire(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    seconds: i64,
) -> Result<(), (StatusCode, String)> {
    match rcm.expire::<&str, i64>(&key, seconds).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("expire: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn exists(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<bool, (StatusCode, String)> {
    match rcm.exists(&key).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("exists: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn set(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref val: &str,
    expiration_time: Option<i64>,
) -> Result<(), (StatusCode, String)> {
    let expiration_time = expiration_time.unwrap_or(EXPIRATION_TIME) as u64;

    match rcm
        .set_ex::<&str, &str, bool>(&key, &val, expiration_time)
        .await
    {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("set: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn incr(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    expiration_time: Option<i64>,
) -> Result<i64, (StatusCode, String)> {
    match rcm.incr(&key, 1).await {
        Ok(amount) => {
            let expiration_time = expiration_time.unwrap_or(EXPIRATION_TIME);
            expire(rcm, &key, expiration_time).await?;

            return Ok(amount);
        }
        Err(e) => {
            error!("incr: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn del(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<(), (StatusCode, String)> {
    match rcm.del::<&str, String>(&key).await {
        Ok(_) => Ok(()),
        Err(e) => {
            error!("del: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn get(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<String, (StatusCode, String)> {
    if !exists(rcm.clone(), &key).await? {
        return Ok("".to_string());
    }

    match rcm.get(&key).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("get: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn sadd(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref val: &str,
    expiration_time: Option<i64>,
) -> Result<(), (StatusCode, String)> {
    match rcm.sadd::<&str, &str, String>(&key, &val).await {
        Ok(_) => {
            expire(rcm, &key, expiration_time.unwrap_or(EXPIRATION_TIME)).await?;
            return Ok(());
        }
        Err(e) => {
            error!("sadd: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn sismember(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref val: &str,
) -> Result<bool, (StatusCode, String)> {
    match rcm.sismember(&key, &val).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("sismember: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn smembers(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<Vec<String>, (StatusCode, String)> {
    match rcm.smembers(&key).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("smembers: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn srem(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref val: &str,
) -> Result<String, (StatusCode, String)> {
    match rcm.srem(&key, &val).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("srem: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn hset_multiple(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref items: &[(&str, &str)],
    expiration_time: Option<i64>,
) -> Result<(), (StatusCode, String)> {
    match rcm
        .hset_multiple::<&str, &str, &str, String>(&key, &items)
        .await
    {
        Ok(_) => {
            expire(rcm, &key, expiration_time.unwrap_or(EXPIRATION_TIME)).await?;

            return Ok(());
        }
        Err(e) => {
            error!("hset_multiple: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn hget(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref field: &str,
) -> Result<String, (StatusCode, String)> {
    match rcm.hget(&key, &field).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("hget: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

pub async fn hgetall(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<Vec<String>, (StatusCode, String)> {
    match rcm.hgetall(&key).await {
        Ok(v) => Ok(v),
        Err(e) => {
            error!("hgetall: {:?}", e);

            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": DB_ERROR_MSG
                })
                .to_string(),
            ));
        }
    }
}

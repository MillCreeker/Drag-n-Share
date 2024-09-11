use axum::{extract::State, http::StatusCode};

use redis::{aio::ConnectionManager, AsyncCommands};

use serde_json::json;

const EXPIRATION_TIME: i64 = 300;

pub async fn expire(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    seconds: i64,
) -> Result<(), (StatusCode, String)> {
    rcm.expire(&key, seconds).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )
    })?;

    Ok(())
}

pub async fn exists(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<bool, (StatusCode, String)> {
    match rcm.exists(&key).await {
        Ok(v) => Ok(v),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )),
    }
}

pub async fn set(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref val: &str,
    expiration_time: Option<i64>,
) -> Result<(), (StatusCode, String)> {
    let expiration_time = expiration_time.unwrap_or(EXPIRATION_TIME) as u64;

    rcm.set_ex::<&str, &str, bool>(&key, &val, expiration_time)
        .await
        .map_err(|_| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                json!({
                    "success": false,
                    "message": "error connection to database"
                })
                .to_string(),
            )
        })?;

    Ok(())
}

pub async fn incr(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    expiration_time: Option<i64>,
) -> Result<i64, (StatusCode, String)> {
    let amount = rcm.incr(&key, 1).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )
    })?;

    let expiration_time = expiration_time.unwrap_or(EXPIRATION_TIME);
    rcm.expire(&key, expiration_time).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )
    })?;

    Ok(amount)
}

pub async fn del(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<(), (StatusCode, String)> {
    rcm.del(&key).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )
    })?;

    Ok(())
}

pub async fn get(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<String, (StatusCode, String)> {
    match rcm.get(&key).await {
        Ok(v) => Ok(v),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )),
    }
}

pub async fn sadd(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref val: &str,
    expiration_time: Option<i64>,
) -> Result<(), (StatusCode, String)> {
    rcm.sadd(&key, &val).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )
    })?;

    expire(rcm, &key, expiration_time.unwrap_or(EXPIRATION_TIME)).await?;

    Ok(())
}

pub async fn sismember(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref val: &str,
) -> Result<bool, (StatusCode, String)> {
    match rcm.sismember(&key, &val).await {
        Ok(v) => Ok(v),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )),
    }
}

pub async fn smembers(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<Vec<String>, (StatusCode, String)> {
    match rcm.smembers(&key).await {
        Ok(v) => Ok(v),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )),
    }
}

pub async fn hset_multiple(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref items: &[(&str, &str)],
    expiration_time: Option<i64>,
) -> Result<(), (StatusCode, String)> {
    rcm.hset_multiple(&key, &items).await.map_err(|_| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )
    })?;

    expire(rcm, &key, expiration_time.unwrap_or(EXPIRATION_TIME)).await?;

    Ok(())
}

pub async fn hget(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
    ref field: &str,
) -> Result<String, (StatusCode, String)> {
    match rcm.hget(&key, &field).await {
        Ok(v) => Ok(v),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )),
    }
}

pub async fn hgetall(
    mut rcm: State<ConnectionManager>,
    ref key: &str,
) -> Result<Vec<String>, (StatusCode, String)> {
    match rcm.hgetall(&key).await {
        Ok(v) => Ok(v),
        Err(_) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            json!({
                "success": false,
                "message": "error connection to database"
            })
            .to_string(),
        )),
    }
}

//! Shared HTTP transport for the web client.

use models::MovieId;
use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn api_base() -> String {
    let hostname = web_sys::window()
        .and_then(|w| w.location().hostname().ok())
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port = std::env::var("server_port").unwrap_or_else(|_| "3000".to_string());
    format!("http://{hostname}:{port}")
}

/// Percent-encode a path segment — ids and locations are opaque.
pub fn encode_segment(raw: &str) -> String {
    raw.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn status_error(status: u16, status_text: &str, body: String) -> String {
    if body.is_empty() {
        format!("{status} {status_text}")
    } else {
        format!("{status}: {body}")
    }
}

pub async fn get_json<T: DeserializeOwned>(path: &str) -> Result<T, String> {
    let url = format!("{}{}", api_base(), path);
    let response = gloo_net::http::Request::get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.ok() {
        let status = response.status();
        let status_text = response.status_text();
        let body = response.text().await.unwrap_or_default();
        return Err(status_error(status, &status_text, body));
    }

    response.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn post_json<B: Serialize, T: DeserializeOwned>(
    path: &str,
    body: &B,
) -> Result<T, String> {
    let url = format!("{}{}", api_base(), path);
    let response = gloo_net::http::Request::post(&url)
        .json(body)
        .map_err(|e| e.to_string())?
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.ok() {
        let status = response.status();
        let status_text = response.status_text();
        let body = response.text().await.unwrap_or_default();
        return Err(status_error(status, &status_text, body));
    }

    response.json::<T>().await.map_err(|e| e.to_string())
}

pub async fn delete_movie(movie_id: MovieId) -> Result<(), String> {
    let url = format!("{}/dvd/{}", api_base(), encode_segment(&movie_id.to_string()));
    let response = gloo_net::http::Request::delete(&url)
        .send()
        .await
        .map_err(|e| format!("Request failed: {e}"))?;

    if !response.ok() {
        let status = response.status();
        let status_text = response.status_text();
        let body = response.text().await.unwrap_or_default();
        return Err(status_error(status, &status_text, body));
    }

    Ok(())
}
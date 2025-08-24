use crate::models::{HostedClipInfo, UserProfile};
use crate::Settings;
use anyhow::Result;
use reqwest::{multipart, Client, Response, StatusCode};
use serde::de::DeserializeOwned;
use std::error::Error;
use std::fmt;
use std::path::Path;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};
use uuid::Uuid;

#[derive(Debug)]
pub enum ApiClientError {
    Unauthorized,
    NotFound,
    ApiError { status: u16, message: String },
    RequestError(reqwest::Error),
    SerializationError(serde_json::Error),
    Io(std::io::Error),
    Config(anyhow::Error),
}

impl fmt::Display for ApiClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiClientError::Unauthorized => write!(f, "Unauthorized: Please log in."),
            ApiClientError::NotFound => write!(f, "The requested resource was not found."),
            ApiClientError::ApiError { status, message } => {
                write!(f, "API Error ({status}): {message}")
            }
            ApiClientError::RequestError(e) => write!(f, "Request Error: {e}"),
            ApiClientError::SerializationError(e) => write!(f, "Serialization Error: {e}"),
            ApiClientError::Io(e) => write!(f, "File I/O Error: {e}"),
            ApiClientError::Config(e) => write!(f, "Configuration Error: {e}"),
        }
    }
}

impl Error for ApiClientError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ApiClientError::RequestError(e) => Some(e),
            ApiClientError::SerializationError(e) => Some(e),
            ApiClientError::Io(e) => Some(e),
            ApiClientError::Config(e) => Some(e.as_ref()),
            _ => None,
        }
    }
}

impl From<anyhow::Error> for ApiClientError {
    fn from(err: anyhow::Error) -> Self {
        ApiClientError::Config(err)
    }
}
impl From<reqwest::Error> for ApiClientError {
    fn from(err: reqwest::Error) -> Self {
        ApiClientError::RequestError(err)
    }
}
impl From<serde_json::Error> for ApiClientError {
    fn from(err: serde_json::Error) -> Self {
        ApiClientError::SerializationError(err)
    }
}
impl From<std::io::Error> for ApiClientError {
    fn from(err: std::io::Error) -> Self {
        ApiClientError::Io(err)
    }
}

async fn handle_response<T: DeserializeOwned>(response: Response) -> Result<T, ApiClientError> {
    match response.status() {
        StatusCode::OK | StatusCode::CREATED => Ok(response.json::<T>().await?),
        StatusCode::UNAUTHORIZED => Err(ApiClientError::Unauthorized),
        StatusCode::NOT_FOUND => Err(ApiClientError::NotFound),
        status => {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not retrieve error message.".to_string());
            Err(ApiClientError::ApiError {
                status: status.as_u16(),
                message,
            })
        }
    }
}

pub async fn get_api_client() -> Result<Client, ApiClientError> {
    let settings = Settings::load().await?;
    let mut headers = reqwest::header::HeaderMap::new();
    if let Some(token) = settings.auth_token {
        headers.insert("Authorization", format!("Bearer {token}").parse().unwrap());
    }
    Ok(Client::builder().default_headers(headers).build()?)
}

pub async fn login(token: String) -> Result<()> {
    let mut settings = Settings::load().await?;
    settings.auth_token = Some(token);
    settings.save().await?;
    Ok(())
}

pub async fn logout() -> Result<()> {
    let mut settings = Settings::load().await?;
    settings.auth_token = None;
    settings.save().await?;
    Ok(())
}

pub async fn get_current_user() -> Result<UserProfile, ApiClientError> {
    let client = get_api_client().await?;
    let settings = Settings::load().await?;
    let response = client
        .get(format!("{}/api/me", settings.api_url))
        .send()
        .await?;
    handle_response(response).await
}

pub async fn get_hosted_clips_index() -> Result<Vec<HostedClipInfo>, ApiClientError> {
    let client = get_api_client().await?;
    let settings = Settings::load().await?;
    let response = client
        .get(format!("{}/api/clips/index", settings.api_url))
        .send()
        .await?;
    handle_response(response).await
}

pub async fn share_clip(client: &Client, clip_path: &Path) -> Result<String, ApiClientError> {
    let settings = Settings::load().await?;
    let file = File::open(clip_path).await?;
    let file_name = clip_path.file_name().unwrap().to_str().unwrap().to_string();

    let stream = FramedRead::new(file, BytesCodec::new());
    let body = reqwest::Body::wrap_stream(stream);

    let part = multipart::Part::stream(body).file_name(file_name);
    let form = multipart::Form::new().part("video", part);

    let response = client
        .post(format!("{}/api/share", settings.api_url))
        .multipart(form)
        .send()
        .await?;

    let response_json: serde_json::Value = handle_response(response).await?;
    let url = response_json["url"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    Ok(url)
}

pub async fn delete_clip(client: &Client, clip_id: Uuid) -> Result<(), ApiClientError> {
    let settings = Settings::load().await?;
    let response = client
        .delete(format!("{}/api/clip/{}", settings.api_url, clip_id))
        .send()
        .await?;

    match response.status() {
        StatusCode::NO_CONTENT => Ok(()),
        StatusCode::UNAUTHORIZED => Err(ApiClientError::Unauthorized),
        StatusCode::NOT_FOUND => Err(ApiClientError::NotFound),
        status => {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not retrieve error message.".to_string());
            Err(ApiClientError::ApiError {
                status: status.as_u16(),
                message,
            })
        }
    }
}

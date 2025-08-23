use crate::models::User;
use crate::Settings;
use anyhow::Result;
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt;

const API_BASE_URL: &str = "http://127.0.0.1:8080";

pub struct ApiClient {
    client: Client,
    auth_token: Option<String>,
}

#[derive(Debug)]
pub enum ApiClientError {
    Unauthorized,
    NotFound,
    ApiError { status: u16, message: String },
    RequestError(reqwest::Error),
    SerializationError(serde_json::Error),
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
        }
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

impl ApiClient {
    pub fn new(auth_token: Option<String>) -> Self {
        Self {
            client: Client::new(),
            auth_token,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{API_BASE_URL}{path}")
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, ApiClientError> {
        let mut request = self.client.get(self.url(path));
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        self.send_request(request).await
    }

    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: B,
    ) -> Result<T, ApiClientError> {
        let mut request = self.client.post(self.url(path));
        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }
        self.send_request(request.json(&body)).await
    }

    async fn send_request<T: DeserializeOwned>(
        &self,
        request_builder: reqwest::RequestBuilder,
    ) -> Result<T, ApiClientError> {
        let response = request_builder.send().await?;

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
}

pub async fn get_me(client: &ApiClient) -> Result<User, ApiClientError> {
    client.get("/api/me").await
}

pub async fn share_clip(client: &ApiClient, clip_name: &str) -> Result<(), ApiClientError> {
    let payload = serde_json::json!({ "name": clip_name });
    client
        .post::<serde_json::Value, _>("/api/share", payload)
        .await?;
    Ok(())
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

pub async fn is_logged_in() -> Result<bool> {
    let settings = Settings::load().await?;
    Ok(settings.auth_token.is_some())
}

pub async fn get_api_client() -> Result<ApiClient> {
    let settings = Settings::load().await?;
    Ok(ApiClient::new(settings.auth_token))
}

pub async fn get_current_user() -> Result<User, ApiClientError> {
    let client = get_api_client()
        .await
        .map_err(|e| ApiClientError::ApiError {
            status: 500,
            message: format!("Failed to load settings: {e}"),
        })?;
    get_me(&client).await
}

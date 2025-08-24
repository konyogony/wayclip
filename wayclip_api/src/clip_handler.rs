use crate::{AppState, settings::Settings};
use actix_files::NamedFile;
use actix_multipart::Multipart;
use actix_web::http::header::ContentType;
use actix_web::{Error, HttpMessage, HttpRequest, HttpResponse, Responder, delete, get, post, web};
use futures_util::stream::StreamExt;
use std::path::PathBuf;
use uuid::Uuid;
use wayclip_core::models::{Clip, HostedClipInfo, User};

const MAX_FILE_SIZE: usize = 1_073_741_824;

#[post("/share")]
pub async fn share_clip(
    req: HttpRequest,
    mut payload: Multipart,
    data: web::Data<AppState>,
    settings: web::Data<Settings>,
) -> Result<HttpResponse, Error> {
    let user_id = req
        .extensions()
        .get::<Uuid>()
        .cloned()
        .ok_or_else(|| actix_web::error::ErrorUnauthorized("Not authenticated"))?;

    let user: User = sqlx::query_as("SELECT * FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_one(&data.db_pool)
        .await
        .map_err(|_| actix_web::error::ErrorNotFound("User not found"))?;

    if user.is_banned {
        return Err(actix_web::error::ErrorForbidden(
            "This account is suspended.",
        ));
    }

    let tier_limit = data.tier_limits.get(&user.tier).cloned().unwrap_or(0);
    let current_usage: i64 =
        sqlx::query_scalar("SELECT COALESCE(SUM(file_size), 0) FROM clips WHERE user_id = $1")
            .bind(user_id)
            .fetch_one(&data.db_pool)
            .await
            .unwrap_or(0);

    if let Some(item) = payload.next().await {
        let mut field = item?;
        let filename = field
            .content_disposition()
            .and_then(|cd| cd.get_filename())
            .unwrap_or("clip.mp4")
            .to_string();

        let mut file_data = Vec::new();
        while let Some(chunk) = field.next().await {
            let data = chunk?;
            if file_data.len() + data.len() > MAX_FILE_SIZE {
                return Err(actix_web::error::ErrorPayloadTooLarge(format!(
                    "File size cannot exceed {MAX_FILE_SIZE} bytes",
                )));
            }
            file_data.extend_from_slice(&data);
        }

        let file_size = file_data.len() as i64;
        if current_usage + file_size > tier_limit {
            return Err(actix_web::error::ErrorForbidden(
                "Storage limit exceeded for your subscription tier.",
            ));
        }

        let storage_path = data
            .storage
            .upload(&filename, file_data)
            .await
            .map_err(|e| {
                tracing::error!("Storage upload failed: {:?}", e);
                actix_web::error::ErrorInternalServerError("Failed to upload file.")
            })?;

        let new_clip: Clip = sqlx::query_as(
            "INSERT INTO clips (user_id, file_name, file_size, public_url) VALUES ($1, $2, $3, $4) RETURNING *",
        )
        .bind(user_id)
        .bind(&filename)
        .bind(file_size)
        .bind(&storage_path)
        .fetch_one(&data.db_pool)
        .await
        .map_err(|e| {
            tracing::error!("DB insert failed: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to save clip metadata.")
        })?;

        let response_url = format!("{}/clip/{}", settings.public_url, new_clip.id);
        Ok(HttpResponse::Ok().json(serde_json::json!({ "url": response_url })))
    } else {
        Err(actix_web::error::ErrorBadRequest("No file uploaded."))
    }
}

#[get("/clip/{id}")]
pub async fn serve_clip(
    req: HttpRequest,
    id: web::Path<Uuid>,
    data: web::Data<AppState>,
    settings: web::Data<Settings>,
) -> impl Responder {
    let clip: Clip = match sqlx::query_as("SELECT * FROM clips WHERE id = $1")
        .bind(*id)
        .fetch_one(&data.db_pool)
        .await
    {
        Ok(c) => c,
        Err(_) => return HttpResponse::NotFound().body("Clip not found."),
    };

    let raw_url = format!("{}/clip/{}/raw", settings.public_url, clip.id);

    let user_agent = req
        .headers()
        .get("User-Agent")
        .and_then(|ua| ua.to_str().ok())
        .unwrap_or("");
    let is_bot = ["Discordbot", "Twitterbot", "facebookexternalhit"]
        .iter()
        .any(|bot| user_agent.contains(bot));

    if is_bot {
        let html = format!(
            r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8" />
                <meta property="og:title" content="A clip from Wayclip" />
                <meta property="og:description" content="File: {}" />
                <meta property="og:type" content="video.other" />
                <meta property="og:video" content="{}" />
                <meta property="og:video:type" content="video/mp4" />
                <meta property="og:video:width" content="1280" />
                <meta property="og:video:height" content="720" />
                <meta name="twitter:card" content="player" />
                <meta name="twitter:title" content="A clip from Wayclip" />
                <meta name="twitter:description" content="File: {}" />
                <meta name="twitter:player" content="{}" />
                <meta name="twitter:player:width" content="1280" />
                <meta name="twitter:player:height" content="720" />
            </head>
            <body>Video shared from Wayclip.</body>
            </html>"#,
            clip.file_name, raw_url, clip.file_name, raw_url
        );
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    } else {
        let report_url = format!("/clip/{}/report", clip.id);
        let html = format!(
            r#"<!DOCTYPE html>
            <html lang="en">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>Wayclip - {}</title>
                <style>
                    body, html {{ margin: 0; padding: 0; height: 100%; font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif; background-color: #161616; color: #e0e0e0; }}
                    .container {{ display: flex; flex-direction: column; height: 100%; }}
                    video {{ width: 100%; flex-grow: 1; object-fit: contain; background-color: #000; }}
                    .footer {{ background-color: #1f1f1f; padding: 12px 20px; text-align: center; border-top: 1px solid #333; }}
                    .footer form button {{ background: #3a3a3a; border: 1px solid #555; color: #fff; padding: 8px 15px; cursor: pointer; border-radius: 5px; font-size: 14px; }}
                    .footer form button:hover {{ background: #4a4a4a; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <video controls autoplay muted playsinline src="{}"></video>
                    <div class="footer">
                        <form action="{}" method="post" onsubmit="this.querySelector('button').disabled=true; this.querySelector('button').innerText='Submitting...';">
                            <button type="submit">Report Clip</button>
                        </form>
                    </div>
                </div>
            </body>
            </html>"#,
            clip.file_name, raw_url, report_url
        );
        HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html)
    }
}

#[get("/clip/{id}/raw")]
pub async fn serve_clip_raw(
    id: web::Path<Uuid>,
    data: web::Data<AppState>,
    settings: web::Data<Settings>,
    req: HttpRequest,
) -> impl Responder {
    let clip: Clip = match sqlx::query_as("SELECT * FROM clips WHERE id = $1")
        .bind(*id)
        .fetch_one(&data.db_pool)
        .await
    {
        Ok(c) => c,
        Err(_) => return HttpResponse::NotFound().finish(),
    };

    if settings.storage_type == "LOCAL" {
        let storage_dir = settings
            .local_storage_path
            .clone()
            .unwrap_or_else(|| "./uploads".to_string());
        let file_path = PathBuf::from(storage_dir).join(&clip.public_url);

        match NamedFile::open(file_path) {
            Ok(file) => file.into_response(&req),
            Err(_) => HttpResponse::NotFound().finish(),
        }
    } else {
        HttpResponse::Found()
            .append_header(("Location", clip.public_url.clone()))
            .finish()
    }
}

#[post("/clip/{id}/report")]
pub async fn report_clip(
    req: HttpRequest,
    id: web::Path<Uuid>,
    data: web::Data<AppState>,
    settings: web::Data<Settings>,
) -> impl Responder {
    let report_data = sqlx::query!(
        r#"
        SELECT c.id as clip_id, c.file_name, u.id as user_id, u.username
        FROM clips c
        JOIN users u ON c.user_id = u.id
        WHERE c.id = $1
        "#,
        *id
    )
    .fetch_one(&data.db_pool)
    .await;

    if let Ok(report) = report_data {
        if let Some(url) = &settings.discord_webhook_url {
            let clip_url = format!("{}/clip/{}", settings.public_url, report.clip_id);
            let reporter_ip = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string();

            let message = serde_json::json!({
                "content": "🚨 New Clip Report!",
                "embeds": [{
                    "title": "Reported Clip Details",
                    "color": 15158332,
                    "fields": [
                        { "name": "Clip URL", "value": clip_url, "inline": false },
                        { "name": "Uploader", "value": format!("{} (`{}`)", report.username, report.user_id), "inline": true },
                        { "name": "Reporter IP", "value": reporter_ip, "inline": true },
                    ]
                }]
            });

            let client = reqwest::Client::new();
            if let Err(e) = client.post(url).json(&message).send().await {
                tracing::error!("Failed to send Discord notification: {}", e);
            }
        }
    }

    let html_content = include_str!("../assets/report.html");
    HttpResponse::Ok()
        .content_type(ContentType::html())
        .body(html_content)
}

#[get("/clips/index")]
pub async fn get_clips_index(req: HttpRequest) -> impl Responder {
    if let Some(user_id) = req.extensions().get::<Uuid>() {
        let data: &web::Data<AppState> = req.app_data().unwrap();
        match sqlx::query_as::<_, HostedClipInfo>(
            "SELECT id, file_name FROM clips WHERE user_id = $1",
        )
        .bind(user_id)
        .fetch_all(&data.db_pool)
        .await
        {
            Ok(clips) => HttpResponse::Ok().json(clips),
            Err(_) => HttpResponse::InternalServerError().body("Could not fetch clips index"),
        }
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

#[delete("/clip/{id}")]
pub async fn delete_clip(
    req: HttpRequest,
    id: web::Path<Uuid>,
    data: web::Data<AppState>,
) -> impl Responder {
    let user_id = match req.extensions().get::<Uuid>() {
        Some(id) => *id,
        None => return HttpResponse::Unauthorized().finish(),
    };

    let clip_to_delete =
        match sqlx::query!("SELECT user_id, public_url FROM clips WHERE id = $1", *id)
            .fetch_optional(&data.db_pool)
            .await
        {
            Ok(Some(clip)) => clip,
            Ok(None) => return HttpResponse::NotFound().finish(),
            Err(_) => return HttpResponse::InternalServerError().finish(),
        };

    if clip_to_delete.user_id != user_id {
        return HttpResponse::NotFound().finish();
    }

    if let Err(e) = data.storage.delete(&clip_to_delete.public_url).await {
        tracing::error!("Failed to delete file from storage: {:?}", e);
    }

    match sqlx::query!(
        "DELETE FROM clips WHERE id = $1 AND user_id = $2",
        *id,
        user_id
    )
    .execute(&data.db_pool)
    .await
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => {
            tracing::error!("Failed to delete clip from database: {:?}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

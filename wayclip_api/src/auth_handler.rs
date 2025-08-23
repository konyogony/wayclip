use crate::{
    AppState, jwt,
    models::{GitHubUser, User},
};
use actix_web::{
    HttpMessage, HttpRequest, HttpResponse, Responder,
    cookie::{Cookie, SameSite},
    get,
    http::header::LOCATION,
    web,
};
use oauth2::reqwest::async_http_client;
use oauth2::{AuthorizationCode, CsrfToken, RedirectUrl, Scope, TokenResponse};
use uuid::Uuid;

#[derive(serde::Deserialize)]
pub struct AuthLoginQuery {
    client: Option<String>,
}

#[get("/github")]
async fn github_login(
    query: web::Query<AuthLoginQuery>,
    data: web::Data<AppState>,
) -> impl Responder {
    let client_type = query.client.as_deref().unwrap_or("web");

    let csrf_token = CsrfToken::new_random();

    let redirect_url = RedirectUrl::new("http://127.0.0.1:8080/auth/callback".to_string()).unwrap();

    let state_with_client = format!("{}:{}", csrf_token.secret(), client_type);
    let csrf_state = CsrfToken::new(state_with_client);

    let (authorize_url, _csrf_state) = data
        .oauth_client
        .clone()
        .set_redirect_uri(redirect_url)
        .authorize_url(|| csrf_state)
        .add_scope(Scope::new("read:user".to_string()))
        .url();

    HttpResponse::Found()
        .append_header((LOCATION, authorize_url.to_string()))
        .finish()
}

#[derive(serde::Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

#[get("/callback")]
async fn github_callback(
    query: web::Query<AuthRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    let state_parts: Vec<&str> = query.state.split(':').collect();
    let client_type = if state_parts.len() == 2 {
        state_parts[1]
    } else {
        "web"
    };

    let code = AuthorizationCode::new(query.code.clone());
    let redirect_url = RedirectUrl::new("http://127.0.0.1:8080/auth/callback".to_string()).unwrap();

    let token_res = data
        .oauth_client
        .clone()
        .set_redirect_uri(redirect_url)
        .exchange_code(code)
        .request_async(async_http_client)
        .await;

    let access_token = match token_res {
        Ok(token) => token.access_token().secret().to_string(),
        Err(e) => return HttpResponse::InternalServerError().body(format!("Error: {e}")),
    };

    let client = reqwest::Client::new();
    let github_user: GitHubUser = match client
        .get("https://api.github.com/user")
        .header("User-Agent", "wayclip-api")
        .bearer_auth(&access_token)
        .send()
        .await
    {
        Ok(res) => match res.json().await {
            Ok(user) => user,
            Err(_) => {
                return HttpResponse::InternalServerError().body("Failed to parse GitHub user");
            }
        },
        Err(_) => return HttpResponse::InternalServerError().body("Failed to fetch GitHub user"),
    };

    let user = match sqlx::query_as::<_, User>(
        "INSERT INTO users (github_id, username, avatar_url) VALUES ($1, $2, $3)
         ON CONFLICT (github_id) DO UPDATE SET username = $2, avatar_url = $3
         RETURNING *",
    )
    .bind(github_user.id)
    .bind(&github_user.login)
    .bind(github_user.avatar_url.as_deref())
    .fetch_one(&data.db_pool)
    .await
    {
        Ok(user) => user,
        Err(e) => return HttpResponse::InternalServerError().body(format!("Database error: {e}")),
    };

    let jwt = match jwt::create_jwt(user.id) {
        Ok(token) => token,
        Err(_) => return HttpResponse::InternalServerError().body("Failed to create token"),
    };

    if client_type == "tauri" {
        let deep_link = format!("wayclip://auth/callback?token={jwt}");
        HttpResponse::Found()
            .append_header((LOCATION, deep_link))
            .finish()
    } else {
        let mut response = HttpResponse::Found();
        response.append_header((LOCATION, "http://localhost:1420"));
        response.cookie(
            Cookie::build("token", jwt)
                .path("/")
                .secure(true)
                .http_only(true)
                .same_site(SameSite::Lax)
                .finish(),
        );
        response.finish()
    }
}

pub async fn get_me(req: HttpRequest) -> impl Responder {
    if let Some(user_id) = req.extensions().get::<Uuid>() {
        let data: &web::Data<AppState> = req.app_data().unwrap();
        match sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&data.db_pool)
            .await
        {
            Ok(user) => HttpResponse::Ok().json(user),
            Err(_) => HttpResponse::NotFound().body("User not found"),
        }
    } else {
        HttpResponse::Unauthorized().finish()
    }
}

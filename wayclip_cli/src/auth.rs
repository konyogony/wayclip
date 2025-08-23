use anyhow::{Context, Result, bail};
use colored::*;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use wayclip_core::api;

pub async fn handle_login() -> Result<()> {
    let (tx, rx) = oneshot::channel::<String>();
    const LOCAL_PORT: u16 = 54321;

    let server_handle = tokio::spawn(async move {
        let listener = match TcpListener::bind(format!("127.0.0.1:{LOCAL_PORT}")).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!(
                    "Error: Could not start local server on port {LOCAL_PORT}. Is another process using it?",
                );
                eprintln!("Details: {e}");
                let _ = tx.send("".to_string());
                return;
            }
        };
        loop {
            if let Ok((mut stream, _)) = listener.accept().await {
                let mut buffer = [0; 2048];
                if stream.read(&mut buffer).await.is_ok() {
                    let request_str = String::from_utf8_lossy(&buffer[..]);
                    if let Some(token) = parse_token_from_request(&request_str) {
                        if tx.send(token).is_err() {
                            break;
                        }
                        let html_content = include_str!("../assets/success.html");
                        let response = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                            html_content.len(),
                            html_content
                        );
                        let _ = stream.write_all(response.as_bytes()).await;
                        let _ = stream.shutdown().await;
                        break;
                    } else {
                        let response = "HTTP/1.1 404 Not Found\r\n\r\n";
                        let _ = stream.write_all(response.as_bytes()).await;
                        let _ = stream.shutdown().await;
                    }
                }
            }
        }
    });

    let redirect_uri = format!("http://127.0.0.1:{LOCAL_PORT}/auth/callback");
    let login_url = format!(
        "http://127.0.0.1:8080/auth/github?client=cli&redirect_uri={}",
        urlencoding::encode(&redirect_uri)
    );

    println!("{}", "○ Opening your browser to complete login...".cyan());
    if opener::open(&login_url).is_err() {
        println!("Could not open browser automatically.");
        println!("Please visit this URL to log in:\n{login_url}");
    }

    println!("{}", "◌ Waiting for authentication...".yellow());
    let token = tokio::time::timeout(Duration::from_secs(120), rx)
        .await
        .context("Login timed out. Please try again.")??;
    server_handle.abort();
    if token.is_empty() {
        bail!("Local server failed to start. Cannot complete login.");
    }
    api::login(token).await?;
    println!("{}", "✔ Login successful!".green().bold());
    Ok(())
}

pub async fn handle_logout() -> Result<()> {
    api::logout().await?;
    println!("{}", "✔ You have been logged out.".green());
    Ok(())
}

fn parse_token_from_request(request: &str) -> Option<String> {
    let first_line = request.lines().next()?;
    if !first_line.contains("/auth/callback") {
        return None;
    }
    let path_and_query = first_line.split_whitespace().nth(1)?;
    let query_string = path_and_query.split('?').nth(1)?;
    let token_param = query_string.split('&').find(|p| p.starts_with("token="))?;
    token_param.strip_prefix("token=").map(String::from)
}

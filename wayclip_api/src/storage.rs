use crate::settings::Settings;
use anyhow::Result;
use async_trait::async_trait;
use ssh2::Session;
use std::io::Write;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use tokio::fs;
use uuid::Uuid;

#[async_trait]
pub trait Storage: Send + Sync {
    async fn upload(&self, file_name: &str, data: Vec<u8>) -> Result<String>;
    async fn delete(&self, storage_path: &str) -> Result<()>;
}

pub struct LocalStorage {
    storage_path: PathBuf,
}

impl LocalStorage {
    pub fn new(settings: &Settings) -> Self {
        let path = settings
            .local_storage_path
            .clone()
            .expect("LOCAL_STORAGE_PATH is required for LOCAL storage type");
        Self {
            storage_path: PathBuf::from(path),
        }
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn upload(&self, file_name: &str, data: Vec<u8>) -> Result<String> {
        if !self.storage_path.exists() {
            fs::create_dir_all(&self.storage_path).await?;
        }

        let extension = Path::new(file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("mp4");
        let unique_filename = format!("{}.{}", Uuid::new_v4(), extension);
        let dest_path = self.storage_path.join(&unique_filename);

        fs::write(dest_path, data).await?;

        Ok(unique_filename)
    }
    async fn delete(&self, storage_path: &str) -> Result<()> {
        let file_path = self.storage_path.join(storage_path);
        if file_path.exists() {
            fs::remove_file(file_path).await?;
        }
        Ok(())
    }
}

pub struct SftpStorage {
    host: String,
    port: u16,
    user: String,
    password: Option<String>,
    remote_path: String,
    public_url: String,
}

impl SftpStorage {
    pub fn new(settings: &Settings) -> Self {
        Self {
            host: settings.sftp_host.clone().expect("SFTP_HOST is required"),
            port: settings.sftp_port.unwrap_or(22),
            user: settings.sftp_user.clone().expect("SFTP_USER is required"),
            password: settings.sftp_password.clone(),
            remote_path: settings
                .sftp_remote_path
                .clone()
                .expect("SFTP_REMOTE_PATH is required"),
            public_url: settings
                .sftp_public_url
                .clone()
                .expect("SFTP_PUBLIC_URL is required"),
        }
    }
}

#[async_trait]
impl Storage for SftpStorage {
    async fn upload(&self, file_name: &str, data: Vec<u8>) -> Result<String> {
        let host = self.host.clone();
        let port = self.port;
        let user = self.user.clone();
        let password = self.password.clone();
        let remote_path_str = self.remote_path.clone();
        let public_url_base = self.public_url.clone();
        let owned_file_name = file_name.to_string();

        tokio::task::spawn_blocking(move || -> Result<String> {
            let tcp = TcpStream::connect(format!("{host}:{port}"))?;
            let mut sess = Session::new()?;
            sess.set_tcp_stream(tcp);
            sess.handshake()?;

            if let Some(pass) = password {
                sess.userauth_password(&user, &pass)?;
            } else {
                panic!("SFTP password authentication is required for this example");
            }

            let sftp = sess.sftp()?;

            let extension = Path::new(&owned_file_name)
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
            let unique_filename = format!("{}.{}", Uuid::new_v4(), extension);
            let remote_file_path = Path::new(&remote_path_str).join(&unique_filename);

            let mut remote_file = sftp.create(remote_file_path.as_path())?;
            remote_file.write_all(&data)?;

            let public_url = format!("{public_url_base}/{unique_filename}");
            Ok(public_url)
        })
        .await?
    }

    async fn delete(&self, storage_path: &str) -> Result<()> {
        let file_name = Path::new(storage_path)
            .file_name()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Could not parse filename from SFTP URL"))?;

        let host = self.host.clone();
        let port = self.port;
        let user = self.user.clone();
        let password = self.password.clone();
        let remote_path_str = self.remote_path.clone();
        let file_name_owned = file_name.to_string();

        tokio::task::spawn_blocking(move || -> Result<()> {
            let tcp = TcpStream::connect(format!("{}:{}", host, port))?;
            let mut sess = Session::new()?;
            sess.set_tcp_stream(tcp);
            sess.handshake()?;

            if let Some(pass) = password {
                sess.userauth_password(&user, &pass)?;
            } else {
                panic!("SFTP password authentication required");
            }

            let sftp = sess.sftp()?;
            let remote_file_path = Path::new(&remote_path_str).join(&file_name_owned);
            sftp.unlink(&remote_file_path)?;
            Ok(())
        })
        .await??;

        Ok(())
    }
}

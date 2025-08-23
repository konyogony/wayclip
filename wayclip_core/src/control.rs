use anyhow::{bail, Context, Result};
use colored::*;
use tokio::process::Command;
use tokio::time::{sleep, Duration};

pub struct DaemonManager;

impl Default for DaemonManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DaemonManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn is_running(&self) -> bool {
        Command::new("systemctl")
            .args(["--user", "is-active", "--quiet", "wayclip-daemon.service"])
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    pub async fn start(&self) -> Result<()> {
        if self.is_running().await {
            bail!("Daemon is already running.");
        }
        println!("Starting daemon via systemd...");
        let output = Command::new("systemctl")
            .args(["--user", "start", "wayclip-daemon.service"])
            .output()
            .await
            .context("Failed to execute systemctl command.")?;

        if !output.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr).red());
            bail!("systemctl failed to start the daemon. Run 'journalctl --user -u wayclip-daemon.service' for details.");
        }

        sleep(Duration::from_millis(500)).await;

        if self.is_running().await {
            println!("{} Daemon started successfully.", "✔".green());
            Ok(())
        } else {
            bail!("Daemon failed to start. Check the logs with 'journalctl --user -u wayclip-daemon.service'.");
        }
    }

    pub async fn stop(&self) -> Result<()> {
        if !self.is_running().await {
            println!("Daemon is not running.");
            return Ok(());
        }
        println!("Stopping daemon via systemd...");
        let output = Command::new("systemctl")
            .args(["--user", "stop", "wayclip-daemon.service"])
            .output()
            .await
            .context("Failed to execute systemctl command.")?;

        if !output.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr).red());
            bail!("systemctl failed to stop the daemon.");
        }
        println!("{} Daemon stopped.", "✔".green());
        Ok(())
    }

    pub async fn restart(&self) -> Result<()> {
        println!("Restarting daemon via systemd...");
        let output = Command::new("systemctl")
            .args(["--user", "restart", "wayclip-daemon.service"])
            .output()
            .await
            .context("Failed to execute systemctl command.")?;

        if !output.status.success() {
            eprintln!("{}", String::from_utf8_lossy(&output.stderr).red());
            bail!("systemctl failed to restart the daemon. Run 'journalctl --user -u wayclip-daemon.service' for details.");
        }

        sleep(Duration::from_millis(500)).await;

        if self.is_running().await {
            println!("{} Daemon restarted successfully.", "✔".green());
            Ok(())
        } else {
            bail!("Daemon failed to enter an active state after restart. Check logs.");
        }
    }

    pub async fn status(&self) -> Result<()> {
        println!("Querying daemon status from systemd...");
        let mut cmd = Command::new("systemctl");
        cmd.args(["--user", "--no-pager", "status", "wayclip-daemon.service"]);

        let status = cmd
            .status()
            .await
            .context("Failed to execute systemctl command. Is systemd running?")?;

        if !status.success() {
            println!(
                "{}",
                "\nDaemon is not running or is in a failed state.".yellow()
            );
        }

        Ok(())
    }
}

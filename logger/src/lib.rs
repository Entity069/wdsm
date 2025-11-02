use chrono::Local;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tracing_subscriber;

pub fn init() {
    tracing_subscriber::fmt::init();
}

fn log_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".wdsm").join("logs")
}

fn flog_dir() -> std::io::Result<()> {
    let dir = log_dir();
    if !dir.exists() {
        fs::create_dir_all(&dir)?;
    }
    Ok(())
}

fn log_to_file(filename: &str, message: &str) {
    if let Err(e) = flog_dir() {
        eprintln!("[!] Failed to create log directory: {}", e);
        return;
    }

    let log_file = log_dir().join(filename);
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
    let log_line = format!("[{}] {}\n", timestamp, message);

    let mut file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
    {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[!] Failed to open log file: {}", e);
            return;
        }
    };

    if let Err(e) = file.write_all(log_line.as_bytes()) {
        eprintln!("[!] Failed to write to log file: {}", e);
    }
}

pub fn log_deployment(deployment_id: &str, event: &str) {
    let message = format!("FnService {} - {}", deployment_id, event);
    log_to_file("deployments.log", &message);
    tracing::info!("{}", message);
}

pub fn log_request(fn_name: &str, input: &str, output: &str) {
    let message = format!("{}: {} -> {}", fn_name, input, output);
    log_to_file("requests.log", &message);
    tracing::debug!("{}", message);
}

pub fn log_error(error: &str) {
    log_to_file("errors.log", error);
    tracing::error!("{}", error);
}
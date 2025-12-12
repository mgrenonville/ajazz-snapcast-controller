// Snapcast Controller Application
// Controls a single room's audio playback via USB HID hardware controller

use std::path::PathBuf;
use std::time::Duration;

use tokio::sync::mpsc::unbounded_channel;

use crate::config::settings::ConnectionSettings;
use crate::hardware::{device::DeviceManager, events::HardwareEvent};

mod config;
mod controller;
mod hardware;
mod snapcast;

#[tokio::main]
async fn main() {
    println!("Snapcast Controller Application");
    println!("Initialization in progress...");

    // Load configuration
    let config_path = get_config_path();
    let config = load_config(&config_path);

    println!("Configuration loaded successfully");
    println!("  Server: {}:{}", config.server.address, config.server.port);
    println!("  Room client ID: {}", config.room.client_id);

    let (tx, _rx) = unbounded_channel::<HardwareEvent>();
    let manager = DeviceManager::new(tx);
    let _ = hardware::device::device_monitor_loop(manager, Duration::from_millis(100)).await;
}

/// Get the configuration file path
/// Checks in order:
/// 1. ./config.toml (current directory)
/// 2. ~/.config/snapcast-controller/config.toml (user config directory)
/// 3. /etc/snapcast-controller/config.toml (system config directory)
fn get_config_path() -> PathBuf {
    // Check current directory
    let local_config = PathBuf::from("config.toml");
    if local_config.exists() {
        return local_config;
    }

    // Check user config directory
    if let Some(home_dir) = dirs::home_dir() {
        let user_config = home_dir
            .join(".config")
            .join("snapcast-controller")
            .join("config.toml");
        if user_config.exists() {
            return user_config;
        }
    }

    // Check system config directory
    let system_config = PathBuf::from("/etc/snapcast-controller/config.toml");
    if system_config.exists() {
        return system_config;
    }

    // Default to local config.toml
    local_config
}

/// Load configuration from file
/// Exits the application if config cannot be loaded
fn load_config(path: &PathBuf) -> ConnectionSettings {
    match ConnectionSettings::load(path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration from {:?}", path);
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Please create a config.toml file with the following format:");
            eprintln!("[server]");
            eprintln!("address = \"192.168.1.100\"");
            eprintln!("port = 1705");
            eprintln!();
            eprintln!("[room]");
            eprintln!("client_id = \"living-room\"");
            std::process::exit(1);
        }
    }
}

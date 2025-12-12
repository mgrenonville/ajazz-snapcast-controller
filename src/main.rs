// Snapcast Controller Application
// Controls a single room's audio playback via USB HID hardware controller

use std::time::Duration;

use tokio::sync::mpsc::unbounded_channel;

use crate::hardware::{device::DeviceManager, events::HardwareEvent};

mod config;
mod controller;
mod hardware;
mod snapcast;

#[tokio::main]
async fn main() {
    println!("Snapcast Controller Application");
    println!("Initialization in progress...");

    let (tx, _rx) = unbounded_channel::<HardwareEvent>();
    let manager = DeviceManager::new(tx);
    let _ = hardware::device::device_monitor_loop(manager, Duration::from_millis(100)).await;
}

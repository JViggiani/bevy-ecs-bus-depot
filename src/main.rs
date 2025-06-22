use bevy::prelude::*;
use ocpp_bevy_poc::app_setup::{setup_bevy_app, AppMode};
use ocpp_bevy_poc::visualization_plugin::log_capture;
use std::env;
use std::fs;
use tracing_subscriber;

fn main() {
    // Determine app mode from command-line arguments.
    // Default to Visual mode if no "--headless" flag is provided.
    let app_mode = if env::args().any(|arg| arg == "--headless") {
        AppMode::Headless
    } else {
        AppMode::Visual
    };

    // Conditionally set up logging. Visual mode gets a channel so we can forward to the UI.
    // Headless mode gets a standard console logger.
    let log_receiver = if app_mode == AppMode::Visual {
        Some(log_capture::setup_logging())
    } else {
        tracing_subscriber::fmt().init();
        None
    };

    info!("Starting Site Controller ECS POC...");

    let config_json = fs::read_to_string("assets/site_config.json")
        .expect("Failed to read site_config.json");
    let (mut app, _app_external_channel_ends) = setup_bevy_app(config_json, app_mode, log_receiver);
    
    // In a real production app, we would now spawn threads/tasks
    // to manage app_external_channel_ends:
    // - app_external_channel_ends.balancer_setpoint_sender_to_bevy:
    //   An external component (e.g., HTTP server, Kafka consumer) would use this
    //   to send setpoints into the Bevy app.
    // - app_external_channel_ends.metering_receiver_from_bevy:
    //   An external component (e.g., Kafka producer, database writer) would use this
    //   to receive metering data from the Bevy app.
    info!("Main application received external channel ends. In a real app, these would be bridged to external systems.");

    app.run();

    info!("Site Controller ECS POC Shutting Down.");
}

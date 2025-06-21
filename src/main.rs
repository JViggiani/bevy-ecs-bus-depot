use bevy::prelude::*;
use ocpp_bevy_poc::app_setup::{setup_bevy_app, AppMode}; 
use std::fs;

fn main() {
    info!("Starting Site Controller ECS POC...");

    let config_json = fs::read_to_string("assets/site_config.json")
        .expect("Failed to read site_config.json");
    let (mut app, _app_external_channel_ends) = setup_bevy_app(config_json, AppMode::Visual);
    
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

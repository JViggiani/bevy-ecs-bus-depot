use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use serde::Deserialize;
use std::collections::HashMap;
use crate::core_asset_plugin::{ExternalId, AssetInfo, TargetPowerSetpointKw, CurrentMeterReading};
use crate::types::EAssetType;
use crate::balancer_comms_plugin::BalancerSetpointData;
use crate::ocpp_protocol_plugin::events::OcppRequestFromChargerEvent;

#[derive(Component, Debug, Clone, Copy)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Component)]
pub struct AssetLabel;

#[derive(Component)]
pub struct Orchestrator;

#[derive(Component)]
pub struct ConnectionLine;

#[derive(Deserialize)]
struct VisualizationConfig {
    positions: HashMap<String, (f32, f32)>,
}

#[derive(Resource, Default)]
struct PositionsAttached(bool);

#[derive(Component)]
pub struct Balancer;

#[derive(Resource)]
struct LogMessages(Vec<String>);

#[derive(Resource)]
pub struct MessageChannels {
    balancer_setpoint_sender: crossbeam_channel::Sender<BalancerSetpointData>,
    ocpp_request_sender: crossbeam_channel::Sender<OcppRequestFromChargerEvent>,
}

#[derive(Resource, Default)]
struct SelectedQueue(String);

#[derive(Resource, Default)]
struct MessageInput(String);

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PositionsAttached(false))
           .insert_resource(LogMessages(Vec::new()))
           .insert_resource(SelectedQueue("Balancer Setpoint".to_string()))
           .insert_resource(MessageInput("{\n  \"external_id\": \"CH001\",\n  \"target_power_kw\": 5.0\n}".to_string()))
           .add_systems(Startup, setup_camera)
           .add_systems(Update, (
               attach_positions_system.run_if(positions_not_attached),
               spawn_asset_visuals_system,
               spawn_orchestrator_system.run_if(orchestrator_not_spawned),
               spawn_balancer_system.run_if(balancer_not_spawned),
               update_asset_colors_system,
               handle_mouse_clicks_system,
               capture_application_logs_system,
               ui_system,
           ));
    }
}

fn positions_not_attached(attached: Res<PositionsAttached>) -> bool {
    !attached.0
}

fn orchestrator_not_spawned(query: Query<&Orchestrator>) -> bool {
    query.is_empty()
}

fn balancer_not_spawned(query: Query<&Balancer>) -> bool {
    query.is_empty()
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

fn attach_positions_system(
    mut commands: Commands,
    mut attached: ResMut<PositionsAttached>,
    query: Query<(Entity, &ExternalId), Without<Position>>,
) {
    let config_str = std::fs::read_to_string("assets/site_visualization.json").unwrap_or_default();
    if config_str.is_empty() {
        warn!("No visualization config found at assets/site_visualization.json");
        return;
    }
    
    let config: VisualizationConfig = serde_json::from_str(&config_str)
        .unwrap_or_else(|e| {
            error!("Failed to parse visualization config: {}", e);
            VisualizationConfig { positions: HashMap::new() }
        });

    info!("Loaded {} position configurations", config.positions.len());
    info!("Found {} entities without Position component", query.iter().count());
    
    for (entity, ext_id) in query.iter() {
        info!("Processing entity {:?} with external_id '{}'", entity, ext_id.0);
        if let Some(&(x, y)) = config.positions.get(&ext_id.0) {
            commands.entity(entity).insert(Position { x, y });
            info!("Added position ({}, {}) to asset '{}'", x, y, ext_id.0);
        } else {
            warn!("No position found for asset '{}'", ext_id.0);
        }
    }
    
    // Mark as attached so this system doesn't run again
    attached.0 = true;
}

#[derive(Component)]
struct Visualized;

fn spawn_asset_visuals_system(
    mut commands: Commands,
    query: Query<(Entity, &Position, &EAssetType, &AssetInfo, &ExternalId), Without<Visualized>>,
) {
    for (entity, pos, asset_type, info, external_id) in query.iter() {
        let color = match asset_type {
            EAssetType::Charger        => Color::srgb(0.2, 0.4, 0.8),
            EAssetType::Battery        => Color::srgb(0.2, 0.8, 0.4),
            EAssetType::GridConnection => Color::srgb(0.8, 0.2, 0.2),
            EAssetType::SolarPV        => Color::srgb(0.9, 0.9, 0.2),
        };

        // Insert the visual components (Sprite, Transform) onto the asset entity.
        commands.entity(entity).insert((
            Sprite::from_color(color, Vec2::new(50.0, 50.0)),
            Transform::from_xyz(pos.x, pos.y, 0.0),
        ));

        // Add text label above the asset
        commands.spawn((
            Text2d::new(format!("{}\n{}", external_id.0, info.model)),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(pos.x, pos.y + 40.0, 1.0),
            AssetLabel,
        ));

        commands.entity(entity).insert(Visualized);
        info!("Spawned visual sprite for '{}' at ({}, {})", info.model, pos.x, pos.y);
    }
}

fn spawn_orchestrator_system(
    mut commands: Commands,
    asset_query: Query<&Position, With<Visualized>>,
) {
    if asset_query.iter().count() < 3 {
        return; // Wait for all assets to be visualized
    }

    // Spawn orchestrator in the center
    commands.spawn((
        Sprite::from_color(Color::srgb(0.8, 0.8, 0.2), Vec2::new(60.0, 60.0)),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Orchestrator,
    ));

    // Add orchestrator label
    commands.spawn((
        Text2d::new("Site Controller\nOrchestrator"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, 50.0, 1.0),
        AssetLabel,
    ));

    // Draw connection lines from orchestrator to each asset
    for position in asset_query.iter() {
        spawn_connection_line(&mut commands, Vec3::ZERO, Vec3::new(position.x, position.y, 0.0));
    }

    info!("Spawned orchestrator and connection lines");
}

fn spawn_balancer_system(
    mut commands: Commands,
    orchestrator_query: Query<&Orchestrator>,
) {
    if orchestrator_query.is_empty() {
        return; // Wait for orchestrator
    }

    // Spawn balancer above orchestrator
    commands.spawn((
        Sprite::from_color(Color::srgb(0.9, 0.5, 0.1), Vec2::new(50.0, 50.0)),
        Transform::from_xyz(0.0, 200.0, 0.0),
        Balancer,
    ));

    // Add balancer label
    commands.spawn((
        Text2d::new("Balancer"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Transform::from_xyz(0.0, 230.0, 1.0),
        AssetLabel,
    ));

    // Draw connection line from balancer to orchestrator
    spawn_connection_line(&mut commands, Vec3::new(0.0, 200.0, 0.0), Vec3::ZERO);

    info!("Spawned balancer");
}

fn ui_system(
    mut contexts: EguiContexts,
    mut selected_queue: ResMut<SelectedQueue>,
    mut message_input: ResMut<MessageInput>,
    mut log_messages: ResMut<LogMessages>,
    channels: Option<Res<MessageChannels>>,
) {
    egui::SidePanel::right("message_interface")
        .default_width(400.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Message Interface");
            ui.separator();

            ui.label("Select Queue:");

            egui::ComboBox::from_label("")
                .selected_text(selected_queue.0.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected_queue.0, "Balancer Setpoint".to_string(), "Balancer Setpoint");
                    ui.selectable_value(&mut selected_queue.0, "OCPP Request".to_string(), "OCPP Request");
                    ui.selectable_value(&mut selected_queue.0, "OCPP Command".to_string(), "OCPP Command");
                });

            if selected_queue.is_changed() {
                message_input.0 = match selected_queue.0.as_str() {
                    "Balancer Setpoint" => "{\n  \"external_id\": \"CH001\",\n  \"target_power_kw\": 5.0\n}".to_string(),
                    "OCPP Request" => "{\n  \"charge_point_id\": \"CH001\",\n  \"action\": \"StatusNotification\",\n  \"payload_json\": \"{}\"\n}".to_string(),
                    _ => "{\n  \"charge_point_id\": \"CH001\",\n  \"message_type\": \"SetChargingProfile\"\n}".to_string(),
                };
                log_messages.0.push(format!("Selected queue: {}", selected_queue.0));
            }

            ui.add_space(10.0);
            ui.label("Message JSON:");
            ui.add(egui::TextEdit::multiline(&mut message_input.0)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .desired_rows(8)
                .desired_width(f32::INFINITY)
            );

            ui.add_space(10.0);
            if ui.button("Send Message").clicked() {
                send_message(&selected_queue.0, &message_input.0, &mut log_messages, channels.as_deref());
            }

            ui.add_space(20.0);
            ui.separator();
            ui.heading("Application Logs");

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    let log_text = log_messages.0.join("\n");
                    ui.label(log_text);
                });
        });
}

fn send_message(
    queue: &str,
    message: &str,
    log_messages: &mut ResMut<LogMessages>,
    channels: Option<&MessageChannels>,
) {
    match queue {
        "Balancer Setpoint" => {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(message) {
                if let (Some(external_id), Some(target_power)) = (
                    data.get("external_id").and_then(|v| v.as_str()),
                    data.get("target_power_kw").and_then(|v| v.as_f64())
                ) {
                    let setpoint = BalancerSetpointData {
                        external_id: external_id.to_string(),
                        target_power_kw: target_power as f32,
                    };
                    
                    if let Some(channels) = channels {
                        if let Err(e) = channels.balancer_setpoint_sender.send(setpoint) {
                            log_messages.0.push(format!("Failed to send setpoint: {}", e));
                        } else {
                            log_messages.0.push(format!("✓ Sent setpoint: {} -> {}kW", external_id, target_power));
                        }
                    } else {
                        log_messages.0.push("⚠ Channels not available - simulation mode".to_string());
                    }
                } else {
                    log_messages.0.push("✗ Invalid setpoint JSON format".to_string());
                }
            } else {
                log_messages.0.push("✗ Invalid JSON format".to_string());
            }
        }
        "OCPP Request" => {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(message) {
                if let (Some(cp_id), Some(action), Some(payload)) = (
                    data.get("charge_point_id").and_then(|v| v.as_str()),
                    data.get("action").and_then(|v| v.as_str()),
                    data.get("payload_json").and_then(|v| v.as_str())
                ) {
                    let request = OcppRequestFromChargerEvent {
                        charge_point_id: cp_id.to_string(),
                        action: action.to_string(),
                        payload_json: payload.to_string(),
                        ocpp_message_id: "ui_msg_1".to_string(),
                    };
                    
                    if let Some(channels) = channels {
                        if let Err(e) = channels.ocpp_request_sender.send(request) {
                            log_messages.0.push(format!("Failed to send OCPP request: {}", e));
                        } else {
                            log_messages.0.push(format!("✓ Sent OCPP {} to {}", action, cp_id));
                        }
                    } else {
                        log_messages.0.push("⚠ Channels not available - simulation mode".to_string());
                    }
                } else {
                    log_messages.0.push("✗ Invalid OCPP request format".to_string());
                }
            } else {
                log_messages.0.push("✗ Invalid JSON format".to_string());
            }
        }
        _ => {
            log_messages.0.push(format!("⚠ Queue '{}' not yet implemented", queue));
        }
    }
}

fn capture_application_logs_system(
    mut log_messages: ResMut<LogMessages>,
    time: Res<Time>,
    mut last_capture: Local<f32>,
    mut heartbeat_counter: Local<u32>,
    // Query for asset activity to create more interesting logs
    asset_query: Query<(&ExternalId, &TargetPowerSetpointKw, &CurrentMeterReading)>,
) {
    // Capture logs every 3 seconds to avoid flooding
    if time.elapsed_secs() - *last_capture > 3.0 {
        *last_capture = time.elapsed_secs();
        *heartbeat_counter += 1;
        
        // Limit log growth
        if log_messages.0.len() > 50 {
            log_messages.0.drain(0..10); // Remove oldest 10 entries
        }
        
        // Add varied application events
        match *heartbeat_counter % 4 {
            0 => {
                let active_assets = asset_query.iter().count();
                log_messages.0.push(format!("System heartbeat #{} - {} assets active", *heartbeat_counter, active_assets));
            }
            1 => {
                // Show asset status
                for (id, setpoint, reading) in asset_query.iter().take(1) {
                    if setpoint.0 > 0.0 || reading.power_kw > 0.0 {
                        log_messages.0.push(format!("Asset {}: {}kW setpoint, {}kW actual", id.0, setpoint.0, reading.power_kw));
                    }
                }
            }
            2 => {
                log_messages.0.push("Checking external connections...".to_string());
            }
            3 => {
                log_messages.0.push("Processing queued messages...".to_string());
            }
            _ => {}
        }
    }
}

fn spawn_connection_line(commands: &mut Commands, start: Vec3, end: Vec3) {
    let direction = end - start;
    let length = direction.length();
    let midpoint = start + direction * 0.5;
    
    commands.spawn((
        Sprite::from_color(Color::srgb(0.5, 0.5, 0.5), Vec2::new(length, 2.0)),
        Transform::from_translation(midpoint)
            .with_rotation(Quat::from_rotation_z(direction.y.atan2(direction.x))),
        ConnectionLine,
    ));
}

fn update_asset_colors_system(
    mut query: Query<(&mut Sprite, &EAssetType, &TargetPowerSetpointKw, &CurrentMeterReading), With<Visualized>>,
) {
    for (mut sprite, asset_type, setpoint, reading) in query.iter_mut() {
        let base_color = match asset_type {
            EAssetType::Charger        => Color::srgb(0.2, 0.4, 0.8),
            EAssetType::Battery        => Color::srgb(0.2, 0.8, 0.4),
            EAssetType::GridConnection => Color::srgb(0.8, 0.2, 0.2),
            EAssetType::SolarPV        => Color::srgb(0.9, 0.9, 0.2),
        };

        // Modify color based on power flow (brighter = more power)
        let power = reading.power_kw.max(setpoint.0);
        let intensity = (power / 10.0).clamp(0.3, 1.0); // Scale 0-10kW to 0.3-1.0 brightness
        
        sprite.color = Color::srgb(
            base_color.to_srgba().red * intensity,
            base_color.to_srgba().green * intensity,
            base_color.to_srgba().blue * intensity,
        );
    }
}

fn handle_mouse_clicks_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    asset_query: Query<(&Position, &ExternalId, &AssetInfo, &TargetPowerSetpointKw, &CurrentMeterReading), With<Visualized>>,
) {
    if !mouse_input.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else {
        return;
    };
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    
    if let Some(cursor_pos) = window.cursor_position() {
        if let Ok(world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_pos) {
            // Check if click is near any asset
            for (pos, external_id, info, setpoint, reading) in asset_query.iter() {
                let asset_pos = Vec2::new(pos.x, pos.y);
                if world_pos.distance(asset_pos) < 30.0 { // Within 30 units
                    info!("Clicked on asset '{}' ({}): Setpoint: {:.1}kW, Reading: {:.1}kW", 
                        external_id.0, info.model, setpoint.0, reading.power_kw);
                }
            }
        }
    }
}

pub fn setup_visualization_channels(
    balancer_sender: crossbeam_channel::Sender<BalancerSetpointData>,
    ocpp_request_sender: crossbeam_channel::Sender<OcppRequestFromChargerEvent>,
) -> MessageChannels {
    MessageChannels {
        balancer_setpoint_sender: balancer_sender,
        ocpp_request_sender,
    }
}
use bevy::prelude::*;
use bevy::render::camera::{Projection, OrthographicProjection};
use bevy_egui::{egui, EguiContexts};
use crate::core_asset_plugin::{ExternalId, AssetInfo, TargetPowerSetpointKw, CurrentMeterReading};
use crate::types::EAssetType;
use crate::balancer_comms_plugin::{BalancerSetpointData, BalancerMeteringData};
use crate::ocpp_protocol_plugin::events::{OcppRequestFromChargerEvent, SendOcppToChargerCommand};
use crate::modbus_protocol_plugin::{ModbusRequest, ModbusResponse};
use crate::asset_template_plugin::TotalAssets;
pub mod log_capture;
use self::log_capture::LogReceiver;
use bevy_pancam::{PanCam, PanCamPlugin};

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

#[derive(Resource, Default)]
struct PositionsAttached(bool);

#[derive(Component)]
pub struct Balancer;

#[derive(Resource, Default)]
struct LogMessages(Vec<String>);

#[derive(Resource, Default)]
struct OutputMessages {
    balancer_metering: Vec<String>,
    ocpp_commands: Vec<String>,
    modbus_requests: Vec<String>,
}

#[derive(Resource)]
pub struct MessageChannels {
    // Senders for input
    balancer_setpoint_sender: crossbeam_channel::Sender<BalancerSetpointData>,
    ocpp_request_sender: crossbeam_channel::Sender<OcppRequestFromChargerEvent>,
    modbus_response_sender: crossbeam_channel::Sender<ModbusResponse>,
    // Receivers for output
    balancer_metering_receiver: crossbeam_channel::Receiver<BalancerMeteringData>,
    ocpp_command_receiver: crossbeam_channel::Receiver<SendOcppToChargerCommand>,
    modbus_request_receiver: crossbeam_channel::Receiver<ModbusRequest>,
}

#[derive(Resource, Default)]
struct SelectedQueue(String);

#[derive(Resource, Default)]
struct MessageInput(String);

pub struct VisualizationPlugin;

impl Plugin for VisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PositionsAttached(false))
           .insert_resource(LogMessages::default())
           .insert_resource(OutputMessages::default())
           .insert_resource(SelectedQueue("Balancer Setpoint".to_string()))
           .insert_resource(MessageInput("{\n  \"external_id\": \"CH001\",\n  \"target_power_kw\": 5.0\n}".to_string()))
           .add_plugins(PanCamPlugin::default())
           .add_systems(Startup, setup_camera)
           .add_systems(Update, (
               attach_positions_system.run_if(positions_not_attached),
               spawn_asset_visuals_system,
               spawn_orchestrator_system.run_if(orchestrator_not_spawned),
               spawn_balancer_system.run_if(balancer_not_spawned),
               update_asset_colors_system,
               handle_mouse_clicks_system,
           ))
           .add_systems(Update, (
               pull_captured_logs_system,
               pull_output_messages_system,
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
    // The near and far planes should be set to allow for sprites on different z-levels.
    let mut projection = OrthographicProjection::default_2d();
    projection.near = -1000.0;
    projection.far = 1000.0;

    // Spawn the camera entity with its core components.
    commands.spawn((
        Camera::default(),
        // The Camera2d component tells Bevy to use the 2D render graph.
        Camera2d::default(),
        // The projection component, which is needed for zooming.
        Projection::Orthographic(projection),
        // Add the PanCam component for panning and zooming.
        PanCam::default(),
    ));
}

fn attach_positions_system(
    mut commands: Commands,
    mut attached: ResMut<PositionsAttached>,
    query: Query<Entity, (With<ExternalId>, Without<Position>)>,
) {
    let assets: Vec<Entity> = query.iter().collect();
    let asset_count = assets.len();
    if asset_count == 0 {
        return;
    }

    let spacing = 250.0;
    let total_width = (asset_count as f32 - 1.0) * spacing;
    let start_x = -total_width / 2.0;

    for (i, entity) in assets.into_iter().enumerate() {
        let x = start_x + i as f32 * spacing;
        let y = -250.0; // Position assets at the bottom
        commands.entity(entity).insert(Position { x, y });
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
    total_assets: Res<TotalAssets>,
) {
    if asset_query.iter().count() < total_assets.0 {
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
        Text2d::new("Orchestrator"),
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
        Transform::from_xyz(0.0, 150.0, 0.0),
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
        Transform::from_xyz(0.0, 180.0, 1.0),
        AssetLabel,
    ));

    // Draw connection line from balancer to orchestrator
    spawn_connection_line(&mut commands, Vec3::new(0.0, 150.0, 0.0), Vec3::ZERO);

    info!("Spawned balancer");
}

/// Pulls captured logs from the receiver channel and adds them to the display buffer.
fn pull_captured_logs_system(
    mut log_messages: ResMut<LogMessages>,
    log_receiver: Option<Res<LogReceiver>>,
) {
    if let Some(receiver) = log_receiver {
        while let Ok(msg) = receiver.0.try_recv() {
            log_messages.0.push(msg.trim_end().to_string());
            // Keep the log buffer from growing indefinitely
            if log_messages.0.len() > 200 {
                log_messages.0.remove(0);
            }
        }
    }
}

/// Pulls messages from the output queues and stores them for display.
fn pull_output_messages_system(
    mut output_messages: ResMut<OutputMessages>,
    channels: Option<Res<MessageChannels>>,
) {
    if let Some(channels) = channels {
        while let Ok(msg) = channels.balancer_metering_receiver.try_recv() {
            output_messages.balancer_metering.push(format!("{:?}", msg));
            if output_messages.balancer_metering.len() > 50 {
                output_messages.balancer_metering.remove(0);
            }
        }
        while let Ok(msg) = channels.ocpp_command_receiver.try_recv() {
            output_messages.ocpp_commands.push(format!("{:?}", msg));
            if output_messages.ocpp_commands.len() > 50 {
                output_messages.ocpp_commands.remove(0);
            }
        }
        while let Ok(msg) = channels.modbus_request_receiver.try_recv() {
            output_messages.modbus_requests.push(format!("{:?}", msg));
            if output_messages.modbus_requests.len() > 50 {
                output_messages.modbus_requests.remove(0);
            }
        }
    }
}

fn ui_system(
    mut contexts: EguiContexts,
    mut selected_queue: ResMut<SelectedQueue>,
    mut message_input: ResMut<MessageInput>,
    log_messages: Res<LogMessages>,
    output_messages: Res<OutputMessages>,
    channels: Option<Res<MessageChannels>>,
) {
    // --- Left Panel (Input) ---
    egui::SidePanel::left("message_interface")
        .default_width(350.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Message Input");
            ui.separator();

            ui.label("Select Queue:");

            egui::ComboBox::from_label("")
                .selected_text(selected_queue.0.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut selected_queue.0, "Balancer Setpoint".to_string(), "Balancer Setpoint");
                    ui.selectable_value(&mut selected_queue.0, "OCPP Request".to_string(), "OCPP Request");
                    ui.selectable_value(&mut selected_queue.0, "Modbus Response".to_string(), "Modbus Response");
                });

            if selected_queue.is_changed() {
                message_input.0 = match selected_queue.0.as_str() {
                    "Balancer Setpoint" => "{\n  \"external_id\": \"CH001\",\n  \"target_power_kw\": 5.0\n}".to_string(),
                    "OCPP Request" => "{\n  \"charge_point_id\": \"CH001\",\n  \"action\": \"MeterValues\",\n  \"payload_json\": \"{\\\"connectorId\\\":1,\\\"meterValue\\\":[{\\\"sampledValue\\\":[{\\\"value\\\":\\\"5000\\\",\\\"measurand\\\":\\\"Power.Active.Import\\\",\\\"unit\\\":\\\"W\\\"}]}]}\"\n}".to_string(),
                    "Modbus Response" => "{\n  \"external_id\": \"BAT001\",\n  \"power_kw\": 5.0,\n  \"energy_kwh\": 1234.5\n}".to_string(),
                    _ => "{}".to_string(),
                };
            }

            ui.add_space(10.0);
            ui.label("Message JSON:");
            ui.add(egui::TextEdit::multiline(&mut message_input.0)
                .font(egui::TextStyle::Monospace)
                .code_editor()
                .desired_rows(10)
                .desired_width(f32::INFINITY)
            );

            ui.add_space(10.0);
            if ui.button("Send Message").clicked() {
                send_message(&selected_queue.0, &message_input.0, channels.as_deref());
            }
        });

    // --- Right Panel (Output) ---
    egui::SidePanel::right("output_queues")
        .default_width(450.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Output Queues");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("Balancer Metering", |ui| {
                    ui.label(output_messages.balancer_metering.join("\n"));
                });
                ui.separator();
                ui.collapsing("OCPP Commands", |ui| {
                    ui.label(output_messages.ocpp_commands.join("\n"));
                });
                ui.separator();
                ui.collapsing("Modbus Requests", |ui| {
                    ui.label(output_messages.modbus_requests.join("\n"));
                });
            });
        });

    // --- Bottom Panel (Logs) ---
    egui::TopBottomPanel::bottom("application_logs")
        .resizable(true)
        .default_height(200.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Application Logs");
            ui.separator();
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
                            error!("Failed to send setpoint: {}", e);
                        } else {
                            info!("Sent setpoint: {} -> {}kW", external_id, target_power);
                        }
                    } else {
                        warn!("Channels not available - simulation mode");
                    }
                } else {
                    error!("Invalid setpoint JSON format");
                }
            } else {
                error!("Invalid JSON format");
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
                            error!("Failed to send OCPP request: {}", e);
                        } else {
                            info!("Sent OCPP {} to {}", action, cp_id);
                        }
                    } else {
                        warn!("Channels not available - simulation mode");
                    }
                } else {
                    error!("Invalid OCPP request format");
                }
            } else {
                error!("Invalid JSON format");
            }
        }
        "Modbus Response" => {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(message) {
                if let (Some(external_id), Some(power_kw), Some(energy_kwh)) = (
                    data.get("external_id").and_then(|v| v.as_str()),
                    data.get("power_kw").and_then(|v| v.as_f64()),
                    data.get("energy_kwh").and_then(|v| v.as_f64()),
                ) {
                    let response = ModbusResponse::new(
                        external_id.to_string(),
                        power_kw as f32,
                        energy_kwh,
                        chrono::Utc::now(),
                    );
                    if let Some(channels) = channels {
                        if let Err(e) = channels.modbus_response_sender.send(response) {
                            error!("Failed to send Modbus response: {}", e);
                        } else {
                            info!("Sent Modbus response for {}", external_id);
                        }
                    } else {
                        warn!("Channels not available - simulation mode");
                    }
                } else {
                    error!("Invalid Modbus response JSON format");
                }
            } else {
                error!("Invalid JSON format");
            }
        }
        _ => {
            warn!("Queue '{}' is invalid", queue);
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
    balancer_setpoint_sender: crossbeam_channel::Sender<BalancerSetpointData>,
    ocpp_request_sender: crossbeam_channel::Sender<OcppRequestFromChargerEvent>,
    modbus_response_sender: crossbeam_channel::Sender<ModbusResponse>,
    balancer_metering_receiver: crossbeam_channel::Receiver<BalancerMeteringData>,
    ocpp_command_receiver: crossbeam_channel::Receiver<SendOcppToChargerCommand>,
    modbus_request_receiver: crossbeam_channel::Receiver<ModbusRequest>,
) -> MessageChannels {
    MessageChannels {
        balancer_setpoint_sender,
        ocpp_request_sender,
        modbus_response_sender,
        balancer_metering_receiver,
        ocpp_command_receiver,
        modbus_request_receiver,
    }
}
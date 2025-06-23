use bevy::prelude::*;
use bevy::render::camera::{Projection, OrthographicProjection};
use bevy_egui::{egui, EguiContexts};
use bevy_pancam::PanCam;

use crate::core_asset_plugin::{ExternalId, AssetInfo, TargetPowerSetpointKw, CurrentMeterReading};
use crate::types::EAssetType;
use crate::balancer_comms_plugin::BalancerSetpointData;
use crate::ocpp_protocol_plugin::events::OcppRequestFromAsset;
use crate::modbus_protocol_plugin::ModbusResponse;
use crate::asset_template_plugin::TotalAssets;

use super::components::{Position, AssetLabel, ConnectionLine, Visualized, OrchestratorVisuals, BalancerVisuals};
use super::log_capture::LogReceiver;
use super::{PositionsAttached, LogMessages, OutputMessages, MessageChannels, SelectedQueue, MessageInput, MessageTemplateLibrary, SelectedTemplate};

pub fn positions_not_attached(attached: Res<PositionsAttached>) -> bool {
    !attached.0
}

// Track whether orchestrator has been spawned to prevent repeat spawning
#[derive(Resource, Default)]
pub struct OrchestratorSpawned(pub bool);

// Track whether balancer has been spawned to prevent repeat spawning
#[derive(Resource, Default)]
pub struct BalancerSpawned(pub bool);

pub fn orchestrator_not_spawned(orchestrator_spawned: Res<OrchestratorSpawned>) -> bool {
    !orchestrator_spawned.0
}

pub fn balancer_not_spawned(balancer_spawned: Res<BalancerSpawned>) -> bool {
    !balancer_spawned.0
}

pub fn setup_camera(mut commands: Commands) {
    // The near and far planes should be set to allow for sprites on different z-levels.
    let mut projection = OrthographicProjection::default_2d();
    projection.near = -1000.0;
    projection.far = 1000.0;

    // Spawn the camera entity with its core components.
    commands.spawn((
        Camera::default(),
        Camera2d::default(),
        Projection::Orthographic(projection),
        PanCam {
            grab_buttons: vec![MouseButton::Middle],
            enabled: true,
            zoom_to_cursor: true,
            min_scale: 0.1,
            max_scale: 10.0,
            ..default()
        },
    ));
}

pub fn attach_positions_system(
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

pub fn spawn_asset_visuals_system(
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

        commands.entity(entity)
            .insert(Sprite {
                color,
                custom_size: Some(Vec2::new(50.0, 50.0)),
                ..default()
            })
            .insert(Transform::from_xyz(pos.x, pos.y, 0.0))
            .insert(GlobalTransform::default())
            .insert(Visibility::default())
            .insert(Visualized);

        commands.spawn((
            Text2d::new(format!("{}\n{}", external_id.0, info.model)),
            TextFont {
                font_size: 12.0,
                ..default()
            },
            TextColor(Color::WHITE),
            Transform::from_xyz(pos.x, pos.y + 40.0, 1.0),
            GlobalTransform::default(),
            Visibility::default(),
            AssetLabel,
        ));

        info!("Spawned visual sprite for '{}' at ({}, {})", info.model, pos.x, pos.y);
    }
}

pub fn spawn_orchestrator_system(
    mut commands: Commands,
    asset_query: Query<&Position, With<Visualized>>,
    total_assets: Res<TotalAssets>,
    mut orchestrator_spawned: ResMut<OrchestratorSpawned>,
) {
    // Only run if not already spawned and all assets are visualized
    if orchestrator_spawned.0 || asset_query.iter().count() < total_assets.0 {
        return;
    }

    // Spawn orchestrator in the center
    commands.spawn((
        Sprite {
            color: Color::srgb(0.8, 0.8, 0.2),
            custom_size: Some(Vec2::new(60.0, 60.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
        OrchestratorVisuals,
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
        GlobalTransform::default(),
        Visibility::default(),
        AssetLabel,
    ));

    // Draw connection lines from orchestrator to each asset
    for position in asset_query.iter() {
        spawn_connection_line(&mut commands, Vec3::ZERO, Vec3::new(position.x, position.y, 0.0));
    }

    // Mark as spawned so this system doesn't run again
    orchestrator_spawned.0 = true;
    info!("Spawned orchestrator and connection lines");
}

pub fn spawn_balancer_system(
    mut commands: Commands,
    orchestrator_query: Query<&OrchestratorVisuals>,
    mut balancer_spawned: ResMut<BalancerSpawned>,
) {
    // Only run if orchestrator exists and balancer not already spawned
    if balancer_spawned.0 || orchestrator_query.is_empty() {
        return;
    }

    // Spawn balancer above orchestrator
    commands.spawn((
        Sprite {
            color: Color::srgb(0.9, 0.5, 0.1),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        Transform::from_xyz(0.0, 150.0, 0.0),
        GlobalTransform::default(),
        Visibility::default(),
        BalancerVisuals,
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
        GlobalTransform::default(),
        Visibility::default(),
        AssetLabel,
    ));

    // Draw connection line from balancer to orchestrator
    spawn_connection_line(&mut commands, Vec3::new(0.0, 150.0, 0.0), Vec3::ZERO);

    // Mark as spawned so this system doesn't run again
    balancer_spawned.0 = true;
    info!("Spawned balancer");
}

fn spawn_connection_line(commands: &mut Commands, start: Vec3, end: Vec3) {
    let direction = end - start;
    let length = direction.length();
    let midpoint = start + direction * 0.5;
    
    commands.spawn_empty()
        .insert(Sprite {
            color: Color::srgb(0.5, 0.5, 0.5),
            custom_size: Some(Vec2::new(length, 2.0)),
            ..default()
        })
        .insert(Transform::from_translation(midpoint)
            .with_rotation(Quat::from_rotation_z(direction.y.atan2(direction.x))))
        .insert(GlobalTransform::default())
        .insert(Visibility::default())
        .insert(ConnectionLine(start, end));
}

pub fn update_asset_colors_system(
    mut query: Query<(&mut Sprite, &EAssetType, &TargetPowerSetpointKw, &CurrentMeterReading), With<Visualized>>,
) {
    for (mut sprite, asset_type, setpoint, reading) in query.iter_mut() {
        // Modify color based on power flow (brighter = more power)
        let power = reading.power_kw.max(setpoint.0);
        let intensity = (power / 10.0).clamp(0.3, 1.0); // Scale 0-10kW to 0.3-1.0 brightness

        let (r, g, b) = match asset_type {
            EAssetType::Charger        => (0.2, 0.4, 0.8),
            EAssetType::Battery        => (0.2, 0.8, 0.4),
            EAssetType::GridConnection => (0.8, 0.2, 0.2),
            EAssetType::SolarPV        => (0.9, 0.9, 0.2),
        };
        
        sprite.color = Color::srgb(
            r * intensity,
            g * intensity,
            b * intensity,
        );
    }
}

pub fn handle_mouse_clicks_system(
    mouse_input: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    camera_query: Query<(&Camera, &GlobalTransform)>,
    asset_query: Query<(&Transform, &ExternalId, &AssetInfo, &TargetPowerSetpointKw, &CurrentMeterReading), With<Visualized>>,
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
            for (transform, external_id, info, setpoint, reading) in asset_query.iter() {
                let asset_pos = transform.translation.truncate();
                if world_pos.distance(asset_pos) < 30.0 { // Within 30 units
                    info!("Clicked on asset '{}' ({}): Setpoint: {:.1}kW, Reading: {:.1}kW", 
                        external_id.0, info.model, setpoint.0, reading.power_kw);
                }
            }
        }
    }
}

pub fn pull_captured_logs_system(
    log_receiver: Option<Res<LogReceiver>>,
    mut log_messages: ResMut<LogMessages>,
) {
    if let Some(receiver) = log_receiver {
        while let Ok(msg) = receiver.0.try_recv() {
            log_messages.0.push(msg.trim_end().to_string());
            if log_messages.0.len() > 200 {
                log_messages.0.remove(0);
            }
        }
    }
}

pub fn pull_output_messages_system(
    channels: Option<Res<MessageChannels>>,
    mut output_messages: ResMut<OutputMessages>,
) {
    if let Some(channels) = channels {
        while let Ok(msg) = channels.balancer_metering_receiver.try_recv() {
            output_messages.balancer_metering.push(format!("{:?}", msg));
            if output_messages.balancer_metering.len() > 50 {
                output_messages.balancer_metering.remove(0);
            }
        }
        while let Ok(msg) = channels.ocpp_to_asset_receiver.try_recv() {
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

pub fn ui_system(
    mut contexts: EguiContexts,
    log_messages: Res<LogMessages>,
    output_messages: Res<OutputMessages>,
    mut selected_queue: ResMut<SelectedQueue>,
    mut selected_template: ResMut<SelectedTemplate>,
    mut message_input: ResMut<MessageInput>,
    template_library: Res<MessageTemplateLibrary>,
    channels: Option<Res<MessageChannels>>,
) {
    // --- Left Panel (Input) ---
    egui::SidePanel::left("message_interface")
        .default_width(350.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Simulate Input to Orchestrator");
            ui.separator();

            ui.label("Select Input Queue:");
            egui::ComboBox::from_label("Queue")
                .selected_text(selected_queue.0.as_str())
                .show_ui(ui, |ui| {
                    for queue_name in template_library.0.keys() {
                        ui.selectable_value(&mut selected_queue.0, queue_name.clone(), queue_name);
                    }
                });

            ui.add_space(10.0);
            ui.label("Select Message Template:");

            if let Some(templates_for_queue) = template_library.0.get(&selected_queue.0) {
                egui::ComboBox::from_label("Template")
                    .selected_text(selected_template.0.as_str())
                    .show_ui(ui, |ui| {
                        for (template_name, _) in templates_for_queue {
                            ui.selectable_value(&mut selected_template.0, template_name.clone(), template_name);
                        }
                    });
            }

            if selected_queue.is_changed() {
                if let Some(templates) = template_library.0.get(&selected_queue.0) {
                    if let Some((first_template_name, first_template_json)) = templates.first() {
                        selected_template.0 = first_template_name.clone();
                        message_input.0 = first_template_json.clone();
                    }
                }
            } else if selected_template.is_changed() {
                if let Some(templates) = template_library.0.get(&selected_queue.0) {
                    if let Some((_, template_json)) = templates.iter().find(|(name, _)| name == &selected_template.0) {
                        message_input.0 = template_json.clone();
                    }
                }
            }

            ui.add_space(10.0);
            ui.label("Message Payload (JSON):");
            ui.add(egui::TextEdit::multiline(&mut message_input.0).code_editor());

            if ui.button("Send Message").clicked() {
                send_message(&message_input.0, &selected_queue.0, &channels);
            }
        });

    // --- Right Panel (Output) ---
    egui::SidePanel::right("output_queues")
        .default_width(450.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Observe Output from Orchestrator");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.collapsing("Output: Balancer Metering", |ui| {
                    ui.label(output_messages.balancer_metering.join("\n"));
                });
                ui.separator();
                ui.collapsing("Output: OCPP Command to Asset", |ui| {
                    ui.label(output_messages.ocpp_commands.join("\n"));
                });
                ui.separator();
                ui.collapsing("Output: Modbus Request to Asset", |ui| {
                    ui.label(output_messages.modbus_requests.join("\n"));
                });
            });
        });

    // --- Bottom Panel (Logs) ---
    egui::TopBottomPanel::bottom("logs")
        .resizable(true)
        .show(contexts.ctx_mut(), |ui| {
            ui.heading("Logs");
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                ui.label(log_messages.0.join("\n"));
            });
        });
}

fn send_message(
    message: &str,
    selected_queue: &str,
    channels: &Option<Res<MessageChannels>>,
) {
    match selected_queue {
        "Balancer Setpoint" => {
            if let Ok(data) = serde_json::from_str::<BalancerSetpointData>(message) {
                if let Some(channels) = channels {
                    if let Err(e) = channels.balancer_setpoint_sender.send(data.clone()) {
                        error!("Failed to send setpoint: {}", e);
                    } else {
                        info!("Sent setpoint: {} -> {}kW", data.external_id, data.target_power_kw);
                    }
                } else {
                    warn!("Channels not available - simulation mode");
                }
            } else {
                error!("Invalid JSON format for BalancerSetpointData");
            }
        }
        "OCPP Request from Asset" => {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(message) {
                if let (Some(cp_id), Some(action), Some(payload)) = (
                    data.get("charge_point_id").and_then(|v| v.as_str()),
                    data.get("action").and_then(|v| v.as_str()),
                    data.get("payload_json").and_then(|v| v.as_str())
                ) {
                    let request = OcppRequestFromAsset {
                        charge_point_id: cp_id.to_string(),
                        action: action.to_string(),
                        payload_json: payload.to_string(),
                        ocpp_message_id: "ui_msg_1".to_string(),
                    };
                    
                    if let Some(channels) = channels {
                        if let Err(e) = channels.ocpp_from_asset_sender.send(request) {
                            error!("Failed to send OCPP request: {}", e);
                        } else {
                            info!("Sent OCPP {} from {}", action, cp_id);
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
        "Modbus Response from Asset" => {
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
                    error!("Invalid Modbus response format");
                }
            } else {
                error!("Invalid JSON format");
            }
        }
        _ => error!("Unknown queue selected: {}", selected_queue),
    }
}
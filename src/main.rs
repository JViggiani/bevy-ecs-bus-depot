use bevy::prelude::*;
use bevy::log::LogPlugin;

mod types;
mod error;

mod core_asset_plugin;
mod ocpp_protocol_plugin;
mod modbus_protocol_plugin;
mod asset_template_plugin;
mod external_comms_plugin;
mod ocpp_sim_task;

use core_asset_plugin::CoreAssetPlugin;
use ocpp_protocol_plugin::OcppProtocolPlugin;
use modbus_protocol_plugin::ModbusProtocolPlugin;
use asset_template_plugin::AssetTemplatePlugin;
use external_comms_plugin::{
    ExternalCommsPlugin, 
    ExternalSetpointSourceForSim, 
    ExternalMeteringSinkForSim,   
};
use ocpp_sim_task::OcppSimSpecificCommandSenders;


const RUN_SIMULATORS: bool = true;

fn main() {
    info!("Starting Site Controller ECS POC...");

    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_duration(std::time::Duration::from_secs(5)))
        .add_plugins(LogPlugin::default())
        .add_plugins(CoreAssetPlugin)
        .add_plugins(OcppProtocolPlugin)
        .add_plugins(ModbusProtocolPlugin)
        .add_plugins(AssetTemplatePlugin)
        .add_plugins(ExternalCommsPlugin {});

    if RUN_SIMULATORS {
        app.init_resource::<OcppSimSpecificCommandSenders>();

        app.add_systems(Startup, (
            spawn_external_balancer_simulator_task, 
        ).chain());
        app.add_systems(Startup, 
            ocpp_sim_task::setup_ocpp_simulation.after(asset_template_plugin::spawn_assets_from_config_system)
        );
        app.add_systems(Update, (
            ocpp_sim_task::poll_ocpp_sim_events_and_fire_bevy_events,
            ocpp_sim_task::forward_bevy_commands_to_sim_thread,
        )); 
    }
    
    app.run();

    info!("Site Controller ECS POC Shutting Down.");
}

fn spawn_external_balancer_simulator_task(
    ext_setpoint_sender: Res<ExternalSetpointSourceForSim>,
    ext_metering_receiver: Res<ExternalMeteringSinkForSim>,
) {
    info!("Spawning External Balancer Simulator Task...");
    let setpoint_sender_clone = ext_setpoint_sender.0.clone();
    let metering_receiver_clone = ext_metering_receiver.0.clone();

    std::thread::spawn(move || {
        info!("External Balancer Simulator Thread Started.");
        let mut counter = 0.0;
        let assets_to_command = vec!["CH001", "CH002", "BAT001"];
        let mut asset_idx = 0;

        loop {
            std::thread::sleep(std::time::Duration::from_secs(7));
            
            let current_asset_id = assets_to_command[asset_idx].to_string();
            let target_power = match current_asset_id.as_str() {
                "CH001" => 7.0 + (counter % 3.0),
                "CH002" => 5.0 - (counter % 2.0),
                "BAT001" => -10.0 + (counter % 10.0),
                _ => 0.0,
            };

            let data = external_comms_plugin::ExternalSetpointData {
                external_id: current_asset_id.clone(),
                target_power_kw: target_power,
            };
            info!("External Balancer Sim: Sending setpoint for {}: {} kW", data.external_id, data.target_power_kw);
            if let Err(e) = setpoint_sender_clone.send(data) {
                error!("External Balancer Sim: Failed to send setpoint: {}", e);
                break; 
            }
            
            counter += 1.0;
            asset_idx = (asset_idx + 1) % assets_to_command.len();

            match metering_receiver_clone.try_recv() {
                Ok(meter_data) => {
                    info!("External Balancer Sim: Received metering for {}: {:.2} kW, {:.2} kWh at {:?}",
                        meter_data.external_id, meter_data.power_kw, meter_data.energy_kwh, meter_data.timestamp);
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {}
                Err(e) => {
                    error!("External Balancer Sim: Error receiving metering: {}", e);
                }
            }
        }
    });
}

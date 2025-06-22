use bevy::prelude::*;
use bevy::log::LogPlugin;
use crate::core_asset_plugin::CoreAssetPlugin;
use crate::asset_template_plugin::AssetTemplatePlugin;
use crate::ocpp_protocol_plugin::{OcppProtocolPlugin, OcppRequestReceiver, OcppCommandSender};
use crate::modbus_protocol_plugin::{ModbusProtocolPlugin, ModbusRequestChannel, ModbusResponseChannel};
use crate::balancer_comms_plugin::{BalancerCommsPlugin, IncomingSetpointChannel, OutgoingMeteringChannel};
use crate::visualization_plugin::VisualizationPlugin;
use crossbeam_channel::{unbounded, Sender, Receiver};
use crate::asset_template_plugin::SiteConfigJson;
use bevy_egui::EguiPlugin;
use crate::visualization_plugin::log_capture::LogReceiver;
use crate::balancer_comms_plugin::{BalancerSetpointData, BalancerMeteringData};
use crate::modbus_protocol_plugin::{ModbusRequest, ModbusResponse};
use crate::ocpp_protocol_plugin::events::{OcppRequestFromChargerEvent, SendOcppToChargerCommand};

/// External channel ends for production integration or tests.
pub struct AppExternalChannelEnds {
    // balancer ↔ Bevy
    pub balancer_setpoint_sender: Sender<crate::balancer_comms_plugin::BalancerSetpointData>,
    pub balancer_setpoint_receiver: Receiver<crate::balancer_comms_plugin::BalancerSetpointData>,
    pub balancer_metering_sender: Sender<crate::balancer_comms_plugin::BalancerMeteringData>,
    pub balancer_metering_receiver: Receiver<crate::balancer_comms_plugin::BalancerMeteringData>,

    // Modbus ↔ Bevy
    pub modbus_request_sender: Sender<crate::modbus_protocol_plugin::ModbusRequest>,
    pub modbus_request_receiver: Receiver<crate::modbus_protocol_plugin::ModbusRequest>,
    pub modbus_response_sender: Sender<crate::modbus_protocol_plugin::ModbusResponse>,
    pub modbus_response_receiver: Receiver<crate::modbus_protocol_plugin::ModbusResponse>,

    // OCPP ↔ Bevy
    pub ocpp_request_sender: Sender<crate::ocpp_protocol_plugin::events::OcppRequestFromChargerEvent>,
    pub ocpp_request_receiver: Receiver<crate::ocpp_protocol_plugin::events::OcppRequestFromChargerEvent>,
    pub ocpp_command_sender: Sender<crate::ocpp_protocol_plugin::events::SendOcppToChargerCommand>,
    pub ocpp_command_receiver: Receiver<crate::ocpp_protocol_plugin::events::SendOcppToChargerCommand>,
}

#[derive(PartialEq, Eq)]
pub enum AppMode {
    Visual,
    Headless,
}

pub fn setup_bevy_app(
    config_json: String,
    mode: AppMode,
    log_receiver: Option<Receiver<String>>,
) -> (App, AppExternalChannelEnds) {
    let mut app = App::new();

    // Balancer channels
    let (balancer_setpoint_sender, balancer_setpoint_receiver) = unbounded::<BalancerSetpointData>();
    let (balancer_metering_sender, balancer_metering_receiver) = unbounded::<BalancerMeteringData>();

    // Modbus channels
    let (modbus_request_sender, modbus_request_receiver) = unbounded::<ModbusRequest>();
    let (modbus_response_sender, modbus_response_receiver) = unbounded::<ModbusResponse>();

    // OCPP channels
    let (ocpp_request_sender, ocpp_request_receiver) = unbounded::<OcppRequestFromChargerEvent>();
    let (ocpp_command_sender, ocpp_command_receiver) = unbounded::<SendOcppToChargerCommand>();

    app.insert_resource(SiteConfigJson(config_json));

    match mode {
        AppMode::Visual => {
            // In visual mode, we use Egui for UI which requires it's own logger, and so we disable the default log plugin.
            app.add_plugins(DefaultPlugins.build().disable::<LogPlugin>());
            app.add_plugins(EguiPlugin {
                   enable_multipass_for_primary_context: false,
               })
               .add_plugins(VisualizationPlugin);
            
            let viz_channels = crate::visualization_plugin::setup_visualization_channels(
                balancer_setpoint_sender.clone(),
                ocpp_request_sender.clone(),
                modbus_response_sender.clone(),
            );
            app.insert_resource(viz_channels);
            if let Some(receiver) = log_receiver {
                app.insert_resource(LogReceiver(receiver));
            }
        }
        AppMode::Headless => {
            // Headless mode uses the standard LogPlugin.
            app.add_plugins(MinimalPlugins);
        }
    }

    app.insert_resource(Time::<Fixed>::from_duration(std::time::Duration::from_secs(5)))
       .add_plugins(CoreAssetPlugin)
       .add_plugins(AssetTemplatePlugin)
       .add_plugins(BalancerCommsPlugin)
       .add_plugins(ModbusProtocolPlugin)
       .add_plugins(OcppProtocolPlugin)

       // insert only the halves needed by ECS/plugin logic:
       .insert_resource(IncomingSetpointChannel(balancer_setpoint_receiver.clone()))
       .insert_resource(OutgoingMeteringChannel(balancer_metering_sender.clone()))
       .insert_resource(ModbusRequestChannel(modbus_request_sender.clone()))
       .insert_resource(ModbusResponseChannel(modbus_response_receiver.clone()))
       .insert_resource(OcppRequestReceiver(ocpp_request_receiver.clone()))
       .insert_resource(OcppCommandSender(ocpp_command_sender.clone()));

    let channels = AppExternalChannelEnds {
        balancer_setpoint_sender,
        balancer_setpoint_receiver,
        balancer_metering_sender,
        balancer_metering_receiver,
        modbus_request_sender,
        modbus_request_receiver,
        modbus_response_sender,
        modbus_response_receiver,
        ocpp_request_sender,
        ocpp_request_receiver,
        ocpp_command_sender,
        ocpp_command_receiver,
    };
    (app, channels)
}

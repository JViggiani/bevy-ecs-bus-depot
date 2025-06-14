use bevy::prelude::*;
use bevy::log::LogPlugin;
use crate::core_asset_plugin::CoreAssetPlugin;
use crate::ocpp_protocol_plugin::OcppProtocolPlugin;
use crate::modbus_protocol_plugin::ModbusProtocolPlugin;
use crate::asset_template_plugin::AssetTemplatePlugin;
use crate::external_comms_plugin::{ExternalCommsPlugin, IncomingSetpointChannel, OutgoingMeteringChannel, ExternalSetpointData, ExternalMeteringData};
use crossbeam_channel::{unbounded, Sender, Receiver};

/// External channel ends for production integration or tests.
pub struct AppExternalChannelEnds {
    pub balancer_setpoint_sender: Sender<ExternalSetpointData>,
    pub metering_receiver: Receiver<ExternalMeteringData>,
}

pub fn setup_bevy_app() -> (App, AppExternalChannelEnds) {
    let mut app = App::new();

    app.add_plugins(MinimalPlugins)
        .insert_resource(Time::<Fixed>::from_duration(std::time::Duration::from_secs(5)))
        .add_plugins(LogPlugin::default())
        .add_plugins(CoreAssetPlugin)
        .add_plugins(OcppProtocolPlugin)
        .add_plugins(ModbusProtocolPlugin)
        .add_plugins(AssetTemplatePlugin);

    let (balancer_setpoint_sender, setpoint_receiver)    = unbounded::<ExternalSetpointData>();
    let (metering_sender, metering_receiver)    = unbounded::<ExternalMeteringData>();

    app.insert_resource(IncomingSetpointChannel(setpoint_receiver));
    app.insert_resource(OutgoingMeteringChannel(metering_sender));
    app.add_plugins(ExternalCommsPlugin {});

    let channels = AppExternalChannelEnds {
        balancer_setpoint_sender,
        metering_receiver,
    };
    (app, channels)
}

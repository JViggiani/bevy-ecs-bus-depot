use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};
use serde::{Deserialize, Serialize};

pub mod events;
pub mod systems;

pub use events::*;
pub use systems::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BalancerSetpointData {
    pub external_id: String,
    pub target_power_kw: f32,
}

#[derive(Debug, Clone)]
pub struct BalancerMeteringData {
    pub external_id: String,
    pub power_kw: f32,
    pub energy_kwh: f64,
    pub timestamp: std::time::SystemTime, // Timestamp of when the metering data was valid/generated
}


#[derive(Default)]
pub struct BalancerCommsPlugin;

impl Plugin for BalancerCommsPlugin {
    fn build(&self, app: &mut App) {
        // Create channels for communication.
        // One pair for setpoints from external balancer INTO Bevy.
        // One pair for metering data FROM Bevy OUT TO external balancer.
        
        app.add_event::<IncomingSetpointEvent>()
            .add_systems(Update, (
                ingest_setpoints_from_channel_system,
                apply_incoming_setpoints_system,
                export_metering_data_to_channel_system,
            ));

        info!("BalancerCommsPlugin loaded.");
    }
}

// Resources to hold the channel ends for Bevy systems
#[derive(Resource)]
pub struct IncomingSetpointChannel(pub Receiver<BalancerSetpointData>);

#[derive(Resource)]
pub struct OutgoingMeteringChannel(pub Sender<BalancerMeteringData>);

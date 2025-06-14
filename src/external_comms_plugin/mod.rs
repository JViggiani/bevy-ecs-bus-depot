use bevy::prelude::*;
use crossbeam_channel::{Receiver, Sender};

pub mod events;
pub mod systems;

pub use events::*;
pub use systems::*;

// Define the data structures that will be sent/received over channels for external communication
#[derive(Debug, Clone)]
pub struct ExternalSetpointData {
    pub external_id: String,
    pub target_power_kw: f32,
}

#[derive(Debug, Clone)]
pub struct ExternalMeteringData {
    pub external_id: String,
    pub power_kw: f32,
    pub energy_kwh: f64,
    pub timestamp: std::time::SystemTime, // Timestamp of when the metering data was valid/generated
}


pub struct ExternalCommsPlugin {
    // These channels are now created in main and passed in, or created here and exposed.
    // For simplicity, let's assume they are created here and corresponding ends stored in resources.
}

impl Plugin for ExternalCommsPlugin {
    fn build(&self, app: &mut App) {
        // Create channels for communication.
        // One pair for setpoints from external balancer INTO Bevy.
        // One pair for metering data FROM Bevy OUT TO external balancer.
        
        // The channels (IncomingSetpointChannel & OutgoingMeteringChannel)
        // are now expected to be inserted as resources before this plugin is added.
        app.add_event::<IncomingSetpointEvent>() // Bevy internal event
            .add_systems(Update, (
                ingest_setpoints_from_channel_system,
                apply_incoming_setpoints_system,
                export_metering_data_to_channel_system,
            ));

        info!("ExternalCommsPlugin loaded.");
    }
}

// Resources to hold the channel ends for Bevy systems
#[derive(Resource)]
pub struct IncomingSetpointChannel(pub Receiver<ExternalSetpointData>);

#[derive(Resource)]
pub struct OutgoingMeteringChannel(pub Sender<ExternalMeteringData>);

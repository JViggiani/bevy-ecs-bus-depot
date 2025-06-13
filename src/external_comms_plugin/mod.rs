use bevy::prelude::*;
use crossbeam_channel::{unbounded, Receiver, Sender};

pub mod events;
pub mod systems;

pub use events::*;
pub use systems::*;

// Define the data structures that will be sent/received over channels
// These are the "Data Transfer Objects" (DTOs) for external communication.
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
        
        // Sender for external systems to send setpoints TO Bevy. Bevy gets the Receiver.
        let (ext_setpoint_sender, bevy_setpoint_receiver) = unbounded::<ExternalSetpointData>();
        // Sender for Bevy to send metering data TO external systems. External systems get the Receiver.
        let (bevy_metering_sender, ext_metering_receiver) = unbounded::<ExternalMeteringData>();

        // Store the Bevy-side channel ends as resources
        app.insert_resource(IncomingSetpointChannel(bevy_setpoint_receiver))
            .insert_resource(OutgoingMeteringChannel(bevy_metering_sender))
            // Expose the other ends for the external (simulator) task to use
            .insert_resource(ExternalSetpointSourceForSim(ext_setpoint_sender))
            .insert_resource(ExternalMeteringSinkForSim(ext_metering_receiver))
            .add_event::<IncomingSetpointEvent>() // Bevy internal event
            .add_systems(Update, (
                ingest_setpoints_from_channel_system,
                apply_incoming_setpoints_system,
                export_metering_data_to_channel_system,
            ));

        // The simulator thread is removed from here. It will be managed in main.rs.
        info!("ExternalCommsPlugin loaded.");
    }
}

// Resources to hold the channel ends for Bevy systems
#[derive(Resource)]
pub struct IncomingSetpointChannel(pub Receiver<ExternalSetpointData>);

#[derive(Resource)]
pub struct OutgoingMeteringChannel(pub Sender<ExternalMeteringData>);

// Resources to hold channel ends for the external simulator task (used in main.rs)
#[derive(Resource, Clone)] // Clone needed if main.rs needs to take ownership for its thread
pub struct ExternalSetpointSourceForSim(pub Sender<ExternalSetpointData>);
#[derive(Resource, Clone)]
pub struct ExternalMeteringSinkForSim(pub Receiver<ExternalMeteringData>);

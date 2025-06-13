use bevy::prelude::*;

#[derive(Event, Debug, Clone)]
pub struct IncomingSetpointEvent {
    pub external_id: String,
    pub target_power_kw: f32,
}

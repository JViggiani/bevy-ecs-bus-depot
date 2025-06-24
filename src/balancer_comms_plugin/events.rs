use bevy::prelude::*;

#[derive(Event, Debug, Clone)]
pub struct SetpointCommand {
    pub entity: Entity,
    pub power_kw: f32,
}

use bevy::prelude::*;
use std::collections::HashMap;

/// After spawning, holds a lookup from config ID → ECS Entity.
#[derive(Resource, Default)]
pub struct ExternalIdMap(pub HashMap<String, Entity>);

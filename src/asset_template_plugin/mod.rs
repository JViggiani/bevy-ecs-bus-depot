use bevy::prelude::*;
pub mod systems;

pub use systems::*;

pub struct AssetTemplatePlugin;

impl Plugin for AssetTemplatePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_assets_from_config_system);
    }
}

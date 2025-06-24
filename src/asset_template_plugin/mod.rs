use bevy::prelude::*;
use crate::common::external_id_map::ExternalIdMap;
pub mod config;
pub mod systems;
pub mod resources;

pub use resources::SiteConfig;
pub use systems::spawn_assets_from_config_system;

#[derive(Resource)]
pub struct TotalAssets(pub usize);

pub struct AssetTemplatePlugin;

impl Plugin for AssetTemplatePlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(ExternalIdMap::default())
           .insert_resource(TotalAssets(0))
           .add_systems(Startup, spawn_assets_from_config_system);
    }
}

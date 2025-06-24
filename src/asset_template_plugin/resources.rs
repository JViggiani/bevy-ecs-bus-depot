use bevy::prelude::Resource;
use crate::asset_template_plugin::config::{AssetInstance, AssetTemplate};
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Resource, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SiteConfig {
    pub asset_templates: HashMap<String, AssetTemplate>,
    pub assets: Vec<AssetInstance>,
}
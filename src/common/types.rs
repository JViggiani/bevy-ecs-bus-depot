use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Component, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub enum EAssetType {
    Charger,
    Battery,
    GridConnection,
    SolarPV,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Component, Reflect, Default)]
#[reflect(Component, Serialize, Deserialize, Default)]
pub enum EOperationalStatus {
    #[default]
    Initializing,
    Online,
    Offline,
    Faulted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Component, Reflect)]
#[reflect(Component, Serialize, Deserialize)]
pub enum EMeteringDataSource {
    Ocpp,
    Modbus,
    InternalCalculation,
}

impl FromStr for EMeteringDataSource {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Ocpp" => Ok(EMeteringDataSource::Ocpp),
            "Modbus" => Ok(EMeteringDataSource::Modbus),
            "InternalCalculation" => Ok(EMeteringDataSource::InternalCalculation),
            _ => Err(()),
        }
    }
}

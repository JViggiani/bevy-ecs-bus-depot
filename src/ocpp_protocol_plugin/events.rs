use bevy::prelude::*;
use crate::types::EOutgoingOcppMessage; 

#[derive(Event, Debug, Clone)]
pub struct OcppRequestFromAsset {
    pub charge_point_id: String,
    pub action: String, 
    pub payload_json: String, 
    pub ocpp_message_id: String, 
}

#[derive(Event, Debug, Clone)]
pub struct OcppCommandToAsset {
    pub charge_point_id: String,
    pub message_type: EOutgoingOcppMessage, 
    pub ocpp_message_id: Option<String>, 
}

#[derive(Resource)]
pub struct OcppFromAssetChannel(pub crossbeam_channel::Receiver<OcppRequestFromAsset>);

#[derive(Resource)]
pub struct OcppToAssetChannel(pub crossbeam_channel::Sender<OcppCommandToAsset>);

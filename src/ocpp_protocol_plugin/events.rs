use bevy::prelude::*;
use crate::types::EOutgoingOcppMessage; 

#[derive(Event, Debug, Clone)]
pub struct OcppRequestFromChargerEvent {
    pub charge_point_id: String,
    pub action: String, 
    pub payload_json: String, 
    pub ocpp_message_id: String, 
}

#[derive(Event, Debug, Clone)]
pub struct SendOcppToChargerCommand {
    pub charge_point_id: String,
    pub message_type: EOutgoingOcppMessage, 
    pub ocpp_message_id: Option<String>, 
}

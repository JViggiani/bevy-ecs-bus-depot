use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum EOcppVersion {
    V1_6J,
}

impl FromStr for EOcppVersion {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "V1_6J" => Ok(EOcppVersion::V1_6J),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum EChargingRateUnit {
    Watts,
    Amps,
}

impl FromStr for EChargingRateUnit {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Watts" => Ok(EChargingRateUnit::Watts),
            "Amps" => Ok(EChargingRateUnit::Amps),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct BootNotificationReqPayload {
    pub charge_point_vendor: String,
    pub charge_point_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, PartialEq, Eq)]
#[reflect(Serialize, Deserialize)]
pub enum RegistrationStatus {
    Accepted,
    Pending,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct BootNotificationConfPayload {
    #[serde(rename = "currentTime")]
    pub current_time: String, 
    pub interval: u32,        
    pub status: RegistrationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct StatusNotificationReqPayload {
    #[serde(rename = "connectorId")]
    pub connector_id: u32,
    #[serde(rename = "errorCode")]
    pub error_code: String, 
    pub status: String, 
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct StatusNotificationConfPayload {
}


#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct MeterSample {
    pub timestamp: Option<String>, 
    #[serde(rename = "sampledValue")]
    pub sampled_value: Vec<MeterValueSampledValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct MeterValueSampledValue {
    pub value: String,
    pub context: Option<String>,   
    pub format: Option<String>,    
    pub measurand: Option<String>, 
    pub phase: Option<String>,     
    pub location: Option<String>,  
    pub unit: Option<String>,      
}


#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct MeterValuesReqPayload {
    #[serde(rename = "connectorId")]
    pub connector_id: u32,
    #[serde(rename = "transactionId")]
    pub transaction_id: Option<i32>, 
    #[serde(rename = "meterValue")]
    pub meter_value: Vec<MeterSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct MeterValuesConfPayload {
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ChargingSchedulePeriod {
    #[serde(rename = "startPeriod")]
    pub start_period: u32, 
    pub limit: f32,        
    #[serde(rename = "numberPhases")]
    pub number_phases: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ChargingSchedule {
    pub duration: Option<u32>, 
    #[serde(rename = "startSchedule")]
    pub start_schedule: Option<String>, 
    #[serde(rename = "chargingRateUnit")]
    pub charging_rate_unit: String, 
    #[serde(rename = "chargingSchedulePeriod")]
    pub charging_schedule_period: Vec<ChargingSchedulePeriod>,
    #[serde(rename = "minChargingRate")]
    pub min_charging_rate: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct CsChargingProfiles {
    #[serde(rename = "chargingProfileId")]
    pub charging_profile_id: i32,
    #[serde(rename = "transactionId")]
    pub transaction_id: Option<i32>,
    #[serde(rename = "stackLevel")]
    pub stack_level: u32,
    #[serde(rename = "chargingProfilePurpose")]
    pub charging_profile_purpose: String, 
    #[serde(rename = "chargingProfileKind")]
    pub charging_profile_kind: String, 
    #[serde(rename = "recurrencyKind")]
    pub recurrency_kind: Option<String>, 
    #[serde(rename = "validFrom")]
    pub valid_from: Option<String>, 
    #[serde(rename = "validTo")]
    pub valid_to: Option<String>, 
    #[serde(rename = "chargingSchedule")]
    pub charging_schedule: ChargingSchedule,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct SetChargingProfileReqPayload {
    #[serde(rename = "connectorId")]
    pub connector_id: u32,
    #[serde(rename = "csChargingProfiles")]
    pub cs_charging_profiles: CsChargingProfiles,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum ChargingProfileStatus {
    Accepted,
    Rejected,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct SetChargingProfileConfPayload {
    pub status: ChargingProfileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct ChangeConfigurationReqPayload {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum ConfigurationStatus {
    Accepted,
    Rejected,
    RebootRequired,
    NotSupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub struct ChangeConfigurationConfPayload {
    pub status: ConfigurationStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect, Default)]
#[reflect(Default, Serialize, Deserialize)]
pub struct RemoteStartTransactionReqPayload {
    #[serde(rename = "connectorId")]
    pub connector_id: Option<u32>,
    #[serde(rename = "idTag")]
    pub id_tag: String,
    #[serde(rename = "chargingProfile")]
    pub charging_profile: Option<CsChargingProfiles>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Reflect)]
#[reflect(Serialize, Deserialize)]
pub enum EOutgoingOcppMessage {
    BootNotificationResponse(BootNotificationConfPayload),
    StatusNotificationResponse(StatusNotificationConfPayload),
    MeterValuesResponse(MeterValuesConfPayload),
    SetChargingProfileRequest(SetChargingProfileReqPayload),
    RemoteStartTransactionRequest(RemoteStartTransactionReqPayload),
    ChangeConfigurationRequest(ChangeConfigurationReqPayload),
}
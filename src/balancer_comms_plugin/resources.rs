use bevy::prelude::Resource;
use super::balancer_messages::{BalancerSetpointMessage, BalancerMeteringMessage};

// External interfaces as resources
#[derive(Resource)]
pub struct BalancerSetpointReceiver(pub crossbeam_channel::Receiver<BalancerSetpointMessage>);

#[derive(Resource)]
pub struct BalancerMeteringSender(pub crossbeam_channel::Sender<BalancerMeteringMessage>);

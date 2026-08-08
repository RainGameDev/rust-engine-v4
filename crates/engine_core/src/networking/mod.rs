use macros::component;

pub mod client;
pub mod packet;
pub mod snapshot;

use crate::ecs::components::Component;

/// Unreliable channel carrying snapshot broadcasts from server to client.
pub const SNAPSHOT_CHANNEL: u8 = 0;
/// Reliable ordered channel carrying client to server messages.
pub const INPUT_CHANNEL: u8 = 2;

#[component(networked)]
pub struct Networked {
    pub id: u64,
}

pub trait Replicate: serde::Serialize + serde::de::DeserializeOwned + Component {}

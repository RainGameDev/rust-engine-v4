use macros::component;

pub mod client;
pub mod packet;
pub mod snapshot;

use crate::ecs::components::Component;

#[component(networked)]
pub struct Networked {
    pub id: u64,
}

pub trait Replicate: serde::Serialize + serde::de::DeserializeOwned + Component {}

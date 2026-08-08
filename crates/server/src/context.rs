use engine_core::ecs::World;
use renet::RenetServer;

use crate::PlayerDirections;

/// Everything a packet handler may touch while serving a tick.
pub struct ServerCtx {
    pub server: RenetServer,
    pub world: World,
    pub directions: PlayerDirections,
}

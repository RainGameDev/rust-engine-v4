use engine_core::ecs::World;
use renet::RenetServer;

use crate::PlayerTargets;

/// Everything a packet handler may touch while serving a tick.
pub struct ServerCtx {
    pub server: RenetServer,
    pub world: World,
    /// Last walk target requested by each client (`None` once arrived).
    pub targets: PlayerTargets,
}

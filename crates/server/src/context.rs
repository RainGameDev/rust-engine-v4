use engine_core::ecs::World;
use engine_core::tiles::TileMap;
use renet::RenetServer;

use crate::PlayerTargets;

/// Everything a packet handler may touch while serving a tick.
pub struct ServerCtx {
    pub server: RenetServer,
    pub world: World,
    /// The authoritative tile map used for spawn points and pathfinding.
    pub map: TileMap,
    /// Last walk target requested by each client (`None` once arrived).
    pub targets: PlayerTargets,
}

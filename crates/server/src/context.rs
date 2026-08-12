use engine_core::ecs::World;
use engine_core::tiles::TileMap;
use game_data::registry::GameRegistry;
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
    /// The data registry for the server (items, quests, npcs ect ect ect)
    pub registry: GameRegistry,
}

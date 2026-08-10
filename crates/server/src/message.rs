use anyhow::Result;
use engine_core::log_warn;
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::networking::INPUT_CHANNEL;
use engine_core::networking::packet::{MoveTo, Packet};

use crate::context::ServerCtx;
use crate::find_player;

/// A client to server packet plus how to apply it to the server world.
pub trait Incoming: Packet {
    fn handle(self, ctx: &mut ServerCtx, client_id: u64);
}

impl Incoming for MoveTo {
    fn handle(self, ctx: &mut ServerCtx, client_id: u64) {
        let Some(player) = find_player(&ctx.world, client_id) else {
            return;
        };
        let Some(transform) = ctx.world.get_component::<Transform>(player) else {
            return;
        };
        let map = &ctx.map;

        // Where the client clicked, in tile space.
        let click = nalgebra::Vector3::new(self.target[0], self.target[1], self.target[2]);
        let (tx, tz) = map.tile_coord(click);
        let Some((wx, wz)) = map.nearest_walkable(tx, tz) else {
            return;
        };

        // A* from the player's current tile to the requested one. Recomputing
        // from the live position makes every new click replace the old path.
        let from = map.tile_coord(transform.position);
        if !map.in_bounds(from.0, from.1) {
            return;
        }
        let Some(tiles) = map.pathfind(from, (wx, wz)) else {
            return;
        };

        let path = tiles
            .iter()
            .filter_map(|&(x, z)| map.tile_center(x, z))
            .collect();
        ctx.targets.insert(client_id, Some(path));
    }
}

type Handler = fn(&mut ServerCtx, u64, &[u8]) -> Result<()>;

fn dispatch<M: Incoming>(ctx: &mut ServerCtx, client_id: u64, payload: &[u8]) -> Result<()> {
    let message = bincode::deserialize::<M>(payload)?;
    M::handle(message, ctx, client_id);
    Ok(())
}

/// One entry per packet type.
/// Add a new packet by implementing `Incoming` for it and registering it here; nothing else changes.
const HANDLERS: &[(u32, Handler)] = &[(MoveTo::ID, dispatch::<MoveTo>)];

pub fn handle_incoming_messages(ctx: &mut ServerCtx) -> Result<()> {
    for client_id in ctx.server.clients_id() {
        while let Some(frame) = ctx.server.receive_message(client_id, INPUT_CHANNEL) {
            let Some(id_bytes) = frame.get(..4) else {
                continue;
            };
            let id = u32::from_le_bytes(id_bytes.try_into().unwrap());
            let Some(payload) = frame.get(4..) else {
                continue;
            };

            let Some((_, handler)) = HANDLERS.iter().find(|(packet_id, _)| *packet_id == id) else {
                log_warn!("client {client_id} sent unknown message id {id}");
                continue;
            };
            handler(ctx, client_id, payload)?;
        }
    }
    Ok(())
}

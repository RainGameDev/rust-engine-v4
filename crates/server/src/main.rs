use std::time::{Duration, Instant};

use anyhow::Result;
use engine_core::ecs::World;
use engine_core::ecs::components::engine_components::transform::transform_update;
use engine_core::ecs::systems::{FixedUpdateSystem, UpdateSystem, run_fixed_update, run_system};
use server::context::ServerCtx;
use server::{
    TICK_RATE_HZ, apply_player_movement, broadcast_snapshots, handle_connection_events,
    message::handle_incoming_messages, setup_networking,
};

fn main() -> Result<()> {
    let world = World::new();
    let (server, mut transport, addr) = setup_networking()?;
    let mut ctx = ServerCtx {
        server,
        world,
        map: engine_core::tiles::TileMap::load_default()?,
        targets: server::PlayerTargets::new(),
    };
    println!("Dedicated server listening on {addr}");

    let tick_duration = Duration::from_secs_f64(1.0 / TICK_RATE_HZ);
    let mut last_tick = Instant::now();

    loop {
        let now = Instant::now();
        let delta = now.duration_since(last_tick);

        if delta >= tick_duration {
            last_tick = now;

            ctx.server.update(delta);
            transport.update(delta, &mut ctx.server)?;

            handle_connection_events(&mut ctx);
            handle_incoming_messages(&mut ctx)?;
            apply_player_movement(&mut ctx);

            run_fixed_update(
                &mut ctx.world,
                FixedUpdateSystem::sorted(),
                tick_duration.as_secs_f32(),
            )?;
            run_system(&mut ctx.world, UpdateSystem::sorted())?;

            // Keep global transforms in sync with movement before snapshotting.
            transform_update(&mut ctx.world);

            broadcast_snapshots(&mut ctx);

            transport.send_packets(&mut ctx.server);
        } else {
            std::thread::sleep(tick_duration - delta);
        }
    }
}

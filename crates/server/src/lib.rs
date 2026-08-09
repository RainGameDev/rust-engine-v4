pub mod context;
pub mod message;

use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::SystemTime;

use anyhow::Result;
use engine_core::ecs::World;
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::ecs::entities::Entity;
use engine_core::ecs::query::query::Query;
use engine_core::networking::Networked;
use engine_core::networking::packet::ServerMessage;
use engine_core::networking::snapshot::build_snapshot;
use nalgebra::Vector3;
use renet::{ConnectionConfig, DefaultChannel, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

use crate::context::ServerCtx;

pub const TICK_RATE_HZ: f64 = 60.0;
const PROTOCOL_ID: u64 = 7;
const MOVE_SPEED: f32 = 30.0;
/// Distance at which a player is considered to have reached their target.
const ARRIVE_RADIUS: f32 = 0.1;

/// Last walk target requested by each connected client.
pub type PlayerTargets = HashMap<u64, Option<Vector3<f32>>>;

pub fn setup_networking() -> Result<(RenetServer, NetcodeServerTransport, std::net::SocketAddr)> {
    let public_addr = "0.0.0.0:5000".parse()?;
    let socket = UdpSocket::bind(public_addr)?;
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

    let server_config = ServerConfig {
        current_time,
        max_clients: 32,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![public_addr],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_config, socket)?;
    let server = RenetServer::new(ConnectionConfig::default());

    Ok((server, transport, public_addr))
}

pub fn handle_connection_events(ctx: &mut ServerCtx) {
    while let Some(event) = ctx.server.get_event() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("client {client_id} connected");
                if find_player(&ctx.world, client_id).is_some() {
                    continue;
                }
                let player = ctx.world.spawn();
                ctx.world.add_component(player, Networked { id: client_id });
                ctx.world
                    .add_component(player, Transform::from_position(Vector3::zeros()));
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("client {client_id} disconnected: {reason}");
                let to_despawn: Vec<Entity> = {
                    let query: Query<(Entity, &Networked)> = Query::new(&ctx.world);

                    for i in query.iter() {
                        println!("{}", i.1.id);
                    }

                    query
                        .iter()
                        .filter(|(_, networked)| networked.id == client_id)
                        .map(|(entity, _)| entity)
                        .collect()
                };
                for entity in to_despawn {
                    ctx.world.despawn(entity);
                }

                ctx.targets.remove(&client_id);
            }
        }
    }
}

pub fn find_player(world: &World, client_id: u64) -> Option<Entity> {
    let query: Query<(Entity, &Networked)> = Query::new(world);
    query
        .iter()
        .find(|(_, networked)| networked.id == client_id)
        .map(|(entity, _)| entity)
}

pub fn apply_player_movement(ctx: &mut ServerCtx) {
    let speed = MOVE_SPEED / TICK_RATE_HZ as f32;
    let mut arrived = Vec::new();

    for (client_id, target) in &ctx.targets {
        let Some(target) = target else {
            continue;
        };
        let Some(player) = find_player(&ctx.world, *client_id) else {
            continue;
        };
        let Some(transform) = ctx.world.get_component_mut::<Transform>(player) else {
            continue;
        };

        let to_target = target - transform.position;
        if to_target.norm_squared() < ARRIVE_RADIUS * ARRIVE_RADIUS {
            arrived.push(*client_id);
            continue;
        }

        let step = to_target.normalize() * speed;
        transform.position += if step.norm_squared() >= to_target.norm_squared() {
            to_target
        } else {
            step
        };
    }

    for client_id in arrived {
        ctx.targets.insert(client_id, None);
    }
}

pub fn broadcast_snapshots(ctx: &mut ServerCtx) {
    let snapshot = build_snapshot(&ctx.world);
    let bytes = match bincode::serialize(&ServerMessage::Snapshot(snapshot)) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("failed to serialize snapshot: {err}");
            return;
        }
    };
    ctx.server
        .broadcast_message(DefaultChannel::Unreliable, bytes);
}

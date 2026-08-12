pub mod context;
pub mod message;

use std::collections::HashMap;
use std::net::UdpSocket;
use std::time::SystemTime;

use anyhow::Result;
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::ecs::entities::Entity;
use engine_core::ecs::query::query::Query;
use engine_core::networking::packet::ServerMessage;
use engine_core::networking::snapshot::build_snapshot;
use engine_core::networking::{Networked, REGISTRY_CHANNEL};
use engine_core::physics::collider::ColliderShape;
use engine_core::{ecs::World, physics::collider::Collider};
use nalgebra::Vector3;
use renet::{ConnectionConfig, DefaultChannel, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

use crate::context::ServerCtx;

pub const TICK_RATE_HZ: f64 = 60.0;
const PROTOCOL_ID: u64 = 7;
const MOVE_SPEED: f32 = 30.0;
/// Distance at which a player is considered to have reached their target.
const ARRIVE_RADIUS: f32 = 0.1;

/// A single waypoint along a player's walk path.
pub type PlayerPath = Vec<Vector3<f32>>;
/// Last walk path requested by each connected client (`None` once arrived).
pub type PlayerTargets = HashMap<u64, Option<PlayerPath>>;

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

pub fn handle_connection_events(ctx: &mut ServerCtx) -> Result<()> {
    while let Some(event) = ctx.server.get_event() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("client {client_id} connected");
                if find_player(&ctx.world, client_id).is_some() {
                    continue;
                }
                let player = ctx.world.spawn();
                ctx.world.add_component(player, Networked { id: client_id });
                let spawn = ctx
                    .map
                    .first_walkable()
                    .and_then(|(x, z)| ctx.map.tile_center(x, z))
                    .unwrap_or(Vector3::zeros());

                let mut transform = Transform::from_position(spawn);
                transform.position.y += 0.5;
                transform.scale = Vector3::new(0.5, 0.5, 0.5);
                ctx.world.add_component(player, transform);

                let collider = Collider::new(
                    ColliderShape::Cuboid {
                        size: Vector3::new(1.0, 1.0, 1.0),
                    },
                    Vector3::new(0.0, 0.0, 0.0),
                );
                ctx.world.add_component(player, collider);

                let bytes = bincode::serialize(&ctx.registry)?;
                let message = ServerMessage::Registry {
                    version: ctx.registry.version,
                    bytes,
                };
                let payload = bincode::serialize(&message)?;
                println!(
                    "sending registry v{} ({} items) to client {client_id}",
                    ctx.registry.version,
                    ctx.registry.items.len()
                );
                ctx.server
                    .send_message(client_id, REGISTRY_CHANNEL, payload);
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

    Ok(())
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

    for (client_id, target) in ctx.targets.iter_mut() {
        let Some(path) = target.as_mut() else {
            continue;
        };
        let Some(player) = find_player(&ctx.world, *client_id) else {
            continue;
        };
        let Some(transform) = ctx.world.get_component_mut::<Transform>(player) else {
            continue;
        };
        while let Some(&waypoint) = path.first() {
            let to_waypoint = waypoint - transform.position;
            if to_waypoint.norm_squared() < ARRIVE_RADIUS * ARRIVE_RADIUS {
                path.remove(0);
                continue;
            }
            let step = to_waypoint.normalize() * speed;
            transform.position += if step.norm_squared() >= to_waypoint.norm_squared() {
                to_waypoint
            } else {
                step
            };
            break;
        }

        if path.is_empty() {
            *target = None;
        }
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

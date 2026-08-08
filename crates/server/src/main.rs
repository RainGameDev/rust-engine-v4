use std::net::UdpSocket;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use engine_core::ecs::World;
use engine_core::ecs::components::engine_components::transform::Transform;
use engine_core::ecs::entities::Entity;
use engine_core::ecs::query::query::Query;
use engine_core::ecs::systems::{
    FixedUpdateSystem, StartSystem, UpdateSystem, run_fixed_update, run_system,
};
use engine_core::networking::Networked;
use engine_core::networking::packet::ClientMessage;
use engine_core::networking::snapshot::build_snapshot;
use nalgebra::Vector3;
use renet::{ConnectionConfig, DefaultChannel, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

const TICK_RATE_HZ: f64 = 60.0;
const PROTOCOL_ID: u64 = 7;
const MOVE_SPEED: f32 = 30.0;

fn main() -> Result<()> {
    let mut world = World::new();
    run_system(&mut world, StartSystem::sorted())?;
    let (mut server, mut transport, addr) = setup_networking()?;
    println!("Dedicated server listening on {addr}");

    let tick_duration = Duration::from_secs_f64(1.0 / TICK_RATE_HZ);
    let mut last_tick = Instant::now();
    let mut directions: HashMap<u64, Vector3<f32>> = HashMap::new();

    loop {
        let now = Instant::now();
        let delta = now.duration_since(last_tick);

        if delta >= tick_duration {
            last_tick = now;

            server.update(delta);
            transport.update(delta, &mut server)?;

            handle_connection_events(&mut server, &mut world, &mut directions);
            handle_incoming_messages(&mut server, &mut directions);
            apply_player_movement(&mut world, &directions);

            run_fixed_update(
                &mut world,
                FixedUpdateSystem::sorted(),
                tick_duration.as_secs_f32(),
            )?;
            run_system(&mut world, UpdateSystem::sorted())?;

            broadcast_snapshots(&mut server, &world);

            transport.send_packets(&mut server);
        } else {
            std::thread::sleep(tick_duration - delta);
        }
    }
}

fn setup_networking() -> Result<(RenetServer, NetcodeServerTransport, std::net::SocketAddr)> {
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

fn handle_connection_events(
    server: &mut RenetServer,
    world: &mut World,
    directions: &mut HashMap<u64, Vector3<f32>>,
) {
    while let Some(event) = server.get_event() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("client {client_id} connected");
                let player = world.spawn();
                world.add_component(player, Networked { id: client_id });
                world.add_component(
                    player,
                    Transform::from_position(Vector3::new(0.0, 0.0, 0.0)),
                );
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("client {client_id} disconnected: {reason}");
                directions.remove(&client_id);
                let to_despawn: Vec<Entity> = {
                    let query: Query<(Entity, &Networked)> = Query::new(world);
                    query
                        .iter()
                        .filter(|(_, networked)| networked.id == client_id)
                        .map(|(entity, _)| entity)
                        .collect()
                };
                for entity in to_despawn {
                    world.despawn(entity);
                }
            }
        }
    }
}

fn find_player(world: &World, client_id: u64) -> Option<Entity> {
    let query: Query<(Entity, &Networked)> = Query::new(world);
    query
        .iter()
        .find(|(_, networked)| networked.id == client_id)
        .map(|(entity, _)| entity)
}

fn handle_incoming_messages(server: &mut RenetServer, directions: &mut HashMap<u64, Vector3<f32>>) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered)
        {
            let client_message: ClientMessage = match bincode::deserialize(&message) {
                Ok(message) => message,
                Err(err) => {
                    println!("failed to deserialize client message: {err}");
                    continue;
                }
            };

            match client_message {
                ClientMessage::PlayerMovement(direction) => {
                    directions.insert(
                        client_id,
                        Vector3::new(direction[0], direction[1], direction[2]),
                    );
                }
            }
        }
    }
}

fn apply_player_movement(world: &mut World, directions: &HashMap<u64, Vector3<f32>>) {
    for (client_id, direction) in directions {
        if *direction == Vector3::zeros() {
            continue;
        }
        let direction = if direction.norm_squared() > 1.0 {
            direction.normalize()
        } else {
            *direction
        };

        let Some(player) = find_player(world, *client_id) else {
            continue;
        };
        if let Some(transform) = world.get_component_mut::<Transform>(player) {
            transform.position += direction * MOVE_SPEED / TICK_RATE_HZ as f32;
        }
    }
}

fn broadcast_snapshots(server: &mut RenetServer, world: &World) {
    let snapshot = build_snapshot(world);
    let bytes = match bincode::serialize(&snapshot) {
        Ok(bytes) => bytes,
        Err(err) => {
            println!("failed to serialize snapshot: {err}");
            return;
        }
    };
    server.broadcast_message(DefaultChannel::Unreliable, bytes);
}

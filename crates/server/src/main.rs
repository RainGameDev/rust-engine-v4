use std::net::UdpSocket;
use std::time::{Duration, Instant, SystemTime};

use anyhow::Result;
use engine_core::ecs::systems::{
    FixedUpdateSystem, StartSystem, UpdateSystem, run_fixed_update, run_system,
};
use renet::{ConnectionConfig, RenetServer, ServerEvent};

use engine_core::ecs::World;
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

const TICK_RATE_HZ: f64 = 30.0;
const PROTOCOL_ID: u64 = 7;

fn main() -> Result<()> {
    let mut world = World::new();
    run_system(&mut world, StartSystem::sorted())?;
    let (mut server, mut transport, addr) = setup_networking()?;
    println!("Dedicated server listening on {addr}");

    let tick_duration = Duration::from_secs_f64(1.0 / TICK_RATE_HZ);
    let mut last_tick = Instant::now();

    loop {
        let now = Instant::now();
        let delta = now.duration_since(last_tick);

        if delta >= tick_duration {
            last_tick = now;

            server.update(delta);
            transport.update(delta, &mut server)?;

            handle_connection_events(&mut server, &mut world);
            handle_incoming_messages(&mut server, &mut world);

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

fn handle_connection_events(server: &mut RenetServer, world: &mut World) {
    while let Some(event) = server.get_event() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("client {client_id} connected");
                // spawn a player entity, add Networked{id}, etc.
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("client {client_id} disconnected: {reason}");
                // despawn their player entity
            }
        }
    }
}

fn handle_incoming_messages(server: &mut RenetServer, world: &mut World) {}

fn broadcast_snapshots(server: &mut RenetServer, world: &World) {}

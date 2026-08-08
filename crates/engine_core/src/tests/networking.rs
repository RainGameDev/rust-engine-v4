use std::net::UdpSocket;
use std::time::{Duration, Instant, SystemTime};

use renet::{
    ConnectionConfig, DefaultChannel, RenetServer, ServerEvent,
};
use renet_netcode::{
    NetcodeServerTransport, ServerAuthentication, ServerConfig,
};

use crate::networking::{
    Networked, client::NetworkClient, packet::ClientMessage, snapshot::{EntitySnapshot, Snapshot},
};

const PROTOCOL_ID: u64 = 7;

fn setup_server() -> (RenetServer, NetcodeServerTransport, std::net::SocketAddr) {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    let local = socket.local_addr().unwrap();
    let current_time = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap();

    let server_config = ServerConfig {
        current_time,
        max_clients: 8,
        protocol_id: PROTOCOL_ID,
        public_addresses: vec![local],
        authentication: ServerAuthentication::Unsecure,
    };

    let transport = NetcodeServerTransport::new(server_config, socket).unwrap();
    let server = RenetServer::new(ConnectionConfig::default());
    (server, transport, local)
}

fn pump_once(
    client: &mut NetworkClient,
    server: &mut RenetServer,
    transport: &mut NetcodeServerTransport,
    delta: Duration,
) {
    client.client.update(delta);
    client
        .transport
        .update(delta, &mut client.client)
        .unwrap();
    server.update(delta);
    transport.update(delta, server).unwrap();
    client.transport.send_packets(&mut client.client);
    transport.send_packets(server);
}

#[test]
fn movement_round_trip() {
    let (mut server, mut transport, addr) = setup_server();
    let mut client = NetworkClient::connect(addr).unwrap();

    let delta = Duration::from_millis(16);

    // 1. Handshake: wait until the server sees the client connect.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut connected = false;
    while Instant::now() < deadline {
        pump_once(&mut client, &mut server, &mut transport, delta);
        while let Some(event) = server.get_event() {
            if matches!(event, ServerEvent::ClientConnected { .. }) {
                connected = true;
            }
        }
        if connected {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(connected, "client never connected to the server");

    // 2. Client -> server: PlayerMovement on the ReliableOrdered channel.
    let message = bincode::serialize(&ClientMessage::PlayerMovement([1.0, 0.0, 0.0])).unwrap();
    client.client.send_message(DefaultChannel::ReliableOrdered, message);

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = false;
    while Instant::now() < deadline {
        pump_once(&mut client, &mut server, &mut transport, delta);
        for client_id in server.clients_id() {
            while let Some(message) =
                server.receive_message(client_id, DefaultChannel::ReliableOrdered)
            {
                let client_message: ClientMessage = bincode::deserialize(&message).unwrap();
                assert!(
                    matches!(client_message, ClientMessage::PlayerMovement(d) if d == [1.0, 0.0, 0.0])
                );
                received = true;
            }
        }
        if received {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(received, "server never received the PlayerMovement");

    // 3. Server -> client: Snapshot on the Unreliable channel.
    let snapshot = Snapshot {
        entities: vec![EntitySnapshot {
            network_id: 1,
            components: vec![("Networked".to_string(), bincode::serialize(&Networked { id: 1 }).unwrap())],
        }],
    };
    server.broadcast_message(
        DefaultChannel::Unreliable,
        bincode::serialize(&snapshot).unwrap(),
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got_snapshot = false;
    while Instant::now() < deadline {
        pump_once(&mut client, &mut server, &mut transport, delta);
        if client.client.is_connected() {
            while let Some(message) = client.client.receive_message(DefaultChannel::Unreliable) {
                let snapshot: Snapshot = bincode::deserialize(&message).unwrap();
                assert_eq!(snapshot.entities.len(), 1);
                assert_eq!(snapshot.entities[0].network_id, 1);
                got_snapshot = true;
            }
        }
        if got_snapshot {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(got_snapshot, "client never received the snapshot");
}

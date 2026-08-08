use std::net::UdpSocket;
use std::time::{Duration, Instant, SystemTime};

use renet::{ConnectionConfig, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};

use crate::networking::{
    INPUT_CHANNEL, Networked, SNAPSHOT_CHANNEL, client::NetworkClient,
    packet::{Packet, PlayerMovement, ServerMessage}, snapshot::{EntitySnapshot, Snapshot},
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
    client.tick(delta).unwrap();
    server.update(delta);
    transport.update(delta, server).unwrap();
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

    // 2. Client -> server: PlayerMovement on the input channel.
    client
        .send(INPUT_CHANNEL, &PlayerMovement { direction: [1.0, 0.0, 0.0] })
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut received = false;
    while Instant::now() < deadline {
        pump_once(&mut client, &mut server, &mut transport, delta);
        for client_id in server.clients_id() {
            while let Some(frame) =
                server.receive_message(client_id, renet::DefaultChannel::ReliableOrdered)
            {
                assert_eq!(u32::from_le_bytes(frame[..4].try_into().unwrap()), PlayerMovement::ID);
                let client_message: PlayerMovement = bincode::deserialize(&frame[4..]).unwrap();
                assert_eq!(client_message.direction, [1.0, 0.0, 0.0]);
                received = true;
            }
        }
        if received {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(received, "server never received the PlayerMovement");

    // 3. Server -> client: Snapshot on the snapshot channel.
    let snapshot = Snapshot {
        entities: vec![EntitySnapshot {
            network_id: 1,
            components: vec![(
                "Networked".to_string(),
                bincode::serialize(&Networked { id: 1 }).unwrap(),
            )],
        }],
    };
    server.broadcast_message(
        renet::DefaultChannel::Unreliable,
        bincode::serialize(&ServerMessage::Snapshot(snapshot)).unwrap(),
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut got_snapshot = false;
    while Instant::now() < deadline {
        pump_once(&mut client, &mut server, &mut transport, delta);
        for message in client.drain::<ServerMessage>(SNAPSHOT_CHANNEL).unwrap() {
            let ServerMessage::Snapshot(snapshot) = message;
            assert_eq!(snapshot.entities.len(), 1);
            assert_eq!(snapshot.entities[0].network_id, 1);
            got_snapshot = true;
        }
        if got_snapshot {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(got_snapshot, "client never received the snapshot");
}

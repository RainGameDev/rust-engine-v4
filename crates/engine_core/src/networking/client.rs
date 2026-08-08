use std::net::UdpSocket;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use macros::Resource;
use renet::{ConnectionConfig, RenetClient};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

use crate::ecs::World;
use crate::networking::SNAPSHOT_CHANNEL;
use crate::networking::packet::{Packet, ServerMessage};
use crate::networking::snapshot::apply_snapshot;

const PROTOCOL_ID: u64 = 7;

/// The client's network connection: transport plus the typed message API.
#[derive(Resource, Debug)]
pub struct NetworkClient {
    client: RenetClient,
    transport: NetcodeClientTransport,
}

impl NetworkClient {
    pub fn connect(server_addr: std::net::SocketAddr) -> Result<Self> {
        let client_id = rand::random::<u64>();
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)?;

        let authentication = ClientAuthentication::Unsecure {
            protocol_id: PROTOCOL_ID,
            client_id,
            server_addr,
            user_data: None,
        };

        let transport = NetcodeClientTransport::new(current_time, authentication, socket)?;
        let client = RenetClient::new(ConnectionConfig::default());

        Ok(Self { client, transport })
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// Advances the renet client and transport, then flushes outgoing packets.
    pub fn tick(&mut self, delta: Duration) -> Result<()> {
        self.client.update(delta);
        self.transport.update(delta, &mut self.client)?;
        let _ = self.transport.send_packets(&mut self.client);
        Ok(())
    }

    /// Serializes and queues a packet for the given channel, framed with its id.
    pub fn send<T: Packet>(&mut self, channel: u8, message: &T) -> Result<()> {
        let payload = bincode::serialize(message)?;
        let mut frame = T::ID.to_le_bytes().to_vec();
        frame.extend_from_slice(&payload);
        self.client.send_message(channel, frame);
        Ok(())
    }

    /// Drains and deserializes every pending message on the given channel.
    pub fn drain<T: serde::de::DeserializeOwned>(&mut self, channel: u8) -> Result<Vec<T>> {
        let mut messages = Vec::new();
        while let Some(message) = self.client.receive_message(channel) {
            messages.push(bincode::deserialize(&message)?);
        }
        Ok(messages)
    }
}

/// One perframe step, pump the transport and apply whatever the server sent.
pub fn pump_network(world: &mut World, delta: Duration) -> Result<()> {
    let messages = {
        let mut network = match world.get_resource_mut::<NetworkClient>() {
            Ok(network) => network,
            Err(_) => return Ok(()),
        };
        network.tick(delta)?;
        network.drain::<ServerMessage>(SNAPSHOT_CHANNEL)?
    };

    for message in messages {
        handle_server_message(world, message);
    }
    Ok(())
}

fn handle_server_message(world: &mut World, message: ServerMessage) {
    match message {
        ServerMessage::Snapshot(snapshot) => apply_snapshot(world, snapshot),
    }
}

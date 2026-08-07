use std::net::UdpSocket;
use std::time::SystemTime;

use anyhow::Result;
use macros::Resource;
use renet::{ConnectionConfig, RenetClient};
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

const PROTOCOL_ID: u64 = 7;
#[derive(Resource, Debug)]
pub struct NetworkClient {
    pub client: RenetClient,
    pub transport: NetcodeClientTransport,
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
}

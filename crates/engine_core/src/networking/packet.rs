use crate::networking::snapshot::Snapshot;

/// A packet is a message with a stable numeric id.
pub trait Packet: serde::Serialize + serde::de::DeserializeOwned + 'static {
    const ID: u32;
}

/// Movement of a player in a vector 3.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PlayerMovement {
    pub direction: [f32; 3],
}

impl Packet for PlayerMovement {
    const ID: u32 = 1;
}

/// Describes something the server is sending to a client.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ServerMessage {
    /// The authoritative state of all networked entities.
    Snapshot(Snapshot),
}

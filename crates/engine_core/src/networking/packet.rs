use crate::networking::snapshot::Snapshot;

/// A packet is a message with a stable numeric id.
pub trait Packet: serde::Serialize + serde::de::DeserializeOwned + 'static {
    const ID: u32;
}

/// A world-space point the player should walk toward.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct MoveTo {
    pub target: [f32; 3],
}

impl Packet for MoveTo {
    const ID: u32 = 1;
}

/// Describes something the server is sending to a client.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ServerMessage {
    /// The authoritative state of all networked entities.
    Snapshot(Snapshot),
}

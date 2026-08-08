/// Describes something the client is sending to the server.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum ClientMessage {
    /// Movement of a player in a vector 3.
    PlayerMovement([f32; 3]),
}

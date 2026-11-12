//! Outgoing Discord message requests shared by the bot and the API server.

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

/// A message the bot should post to a Discord channel.
#[derive(Debug)]
#[non_exhaustive]
pub struct OutgoingMessage {
    /// Channel that should receive the message.
    pub channel_id: String,
    /// Raw Discord message payload, forwarded unchanged.
    pub payload: Value,
    /// Reports the sent message ID or a human-readable failure.
    pub reply: oneshot::Sender<Result<String, String>>,
}

impl OutgoingMessage {
    /// Build a message and the receiver for its resulting message ID.
    #[must_use]
    pub fn new(
        channel_id: impl Into<String>,
        payload: Value,
    ) -> (Self, oneshot::Receiver<Result<String, String>>) {
        let (reply, response) = oneshot::channel();

        (
            Self {
                channel_id: channel_id.into(),
                payload,
                reply,
            },
            response,
        )
    }
}

/// Sender half of the bot's outgoing message queue.
pub type MessageSender = mpsc::Sender<OutgoingMessage>;

use serde::{Deserialize, Serialize};

use crate::controls::ControlState;

pub const MAX_PACKET_SIZE: usize = 2048;

#[derive(Debug, Serialize, Deserialize)]
pub enum ClientPacket {
    Join {
        desired_slot: Option<u8>,
        name: String,
    },
    Input {
        slot: u8,
        state: ControlState,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ServerPacket {
    Ack { slot: u8, total_slots: u8 },
    Reject { reason: String },
}

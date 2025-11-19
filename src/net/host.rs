use std::io::{self, ErrorKind};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use super::protocol::{ClientPacket, MAX_PACKET_SIZE, ServerPacket};
use crate::controls::ControlState;

const SLOT_TIMEOUT: Duration = Duration::from_secs(5);
const LOCAL_SLOT: usize = 0;

#[derive(Clone, Default)]
struct SlotInfo {
    addr: Option<SocketAddr>,
    name: Option<String>,
    last_seen: Option<Instant>,
}

pub struct NetworkInputHost {
    socket: UdpSocket,
    buffer: [u8; MAX_PACKET_SIZE],
    slots: Vec<SlotInfo>,
    states: Vec<ControlState>,
}

impl NetworkInputHost {
    pub fn bind(bind_addr: &str) -> io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_nonblocking(true)?;
        Ok(Self {
            socket,
            buffer: [0_u8; MAX_PACKET_SIZE],
            slots: Vec::new(),
            states: Vec::new(),
        })
    }

    pub fn configure_slots(&mut self, count: usize) {
        self.slots = (0..count).map(|_| SlotInfo::default()).collect();
        self.states = (0..count).map(|_| ControlState::idle()).collect();
    }

    pub fn poll(&mut self) {
        self.cleanup_timeouts();
        loop {
            match self.socket.recv_from(&mut self.buffer) {
                Ok((len, addr)) => {
                    if let Ok(packet) = bincode::deserialize::<ClientPacket>(&self.buffer[..len]) {
                        self.handle_packet(packet, addr);
                    }
                }
                Err(err) if err.kind() == ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }

    pub fn fill_states(&self, target: &mut [ControlState]) {
        for (idx, state) in self.states.iter().enumerate() {
            if let Some(slot) = target.get_mut(idx) {
                *slot = *state;
            }
        }
    }

    pub fn state_for(&self, index: usize) -> ControlState {
        self.states
            .get(index)
            .copied()
            .unwrap_or_else(ControlState::idle)
    }

    fn handle_packet(&mut self, packet: ClientPacket, addr: SocketAddr) {
        match packet {
            ClientPacket::Join { desired_slot, name } => self.handle_join(addr, desired_slot, name),
            ClientPacket::Input { slot, state } => self.handle_input(addr, slot as usize, state),
        }
    }

    fn handle_join(&mut self, addr: SocketAddr, desired: Option<u8>, name: String) {
        if self.slots.is_empty() {
            let _ = self.send_packet(
                addr,
                &ServerPacket::Reject {
                    reason: "game not ready".to_string(),
                },
            );
            return;
        }

        let slot = desired
            .and_then(|value| self.validate_slot(value as usize))
            .or_else(|| self.first_available_slot());

        match slot {
            Some(idx) => {
                if let Some(info) = self.slots.get_mut(idx) {
                    info.addr = Some(addr);
                    info.name = Some(name);
                    info.last_seen = Some(Instant::now());
                    if let Some(state) = self.states.get_mut(idx) {
                        *state = ControlState::idle();
                    }
                    let total = self.states.len() as u8;
                    let _ = self.send_packet(
                        addr,
                        &ServerPacket::Ack {
                            slot: idx as u8,
                            total_slots: total,
                        },
                    );
                }
            }
            None => {
                let _ = self.send_packet(
                    addr,
                    &ServerPacket::Reject {
                        reason: "no available slots".to_string(),
                    },
                );
            }
        }
    }

    fn handle_input(&mut self, addr: SocketAddr, slot: usize, state: ControlState) {
        if slot == LOCAL_SLOT {
            return;
        }
        if let Some(info) = self.slots.get_mut(slot) {
            match info.addr {
                Some(current) if current == addr => {
                    if let Some(stored) = self.states.get_mut(slot) {
                        *stored = state;
                    }
                    info.last_seen = Some(Instant::now());
                }
                _ => {}
            }
        }
    }

    fn validate_slot(&self, slot: usize) -> Option<usize> {
        if slot == LOCAL_SLOT || slot >= self.slots.len() {
            return None;
        }
        match self.slots.get(slot) {
            Some(info) if info.addr.is_none() => Some(slot),
            _ => None,
        }
    }

    fn first_available_slot(&self) -> Option<usize> {
        self.slots
            .iter()
            .enumerate()
            .skip(LOCAL_SLOT + 1)
            .find(|(_, info)| info.addr.is_none())
            .map(|(idx, _)| idx)
    }

    fn cleanup_timeouts(&mut self) {
        let now = Instant::now();
        for (idx, info) in self.slots.iter_mut().enumerate() {
            if idx == LOCAL_SLOT {
                continue;
            }
            if let Some(last) = info.last_seen {
                if now.duration_since(last) > SLOT_TIMEOUT {
                    info.addr = None;
                    info.name = None;
                    info.last_seen = None;
                    if let Some(state) = self.states.get_mut(idx) {
                        *state = ControlState::idle();
                    }
                }
            }
        }
    }

    fn send_packet(&self, addr: SocketAddr, packet: &ServerPacket) -> io::Result<()> {
        let bytes = bincode::serialize(packet).unwrap_or_default();
        self.socket.send_to(&bytes, addr)?;
        Ok(())
    }
}

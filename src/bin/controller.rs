use std::io::{self, ErrorKind};
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

use macroquad::prelude::*;

use my_game::controls::{ControlState, SkillCommand};
use my_game::net::protocol::{ClientPacket, MAX_PACKET_SIZE, ServerPacket};
use my_game::skills::SkillKind;

const RESEND_INTERVAL: Duration = Duration::from_secs(1);

struct ControllerClient {
    socket: UdpSocket,
    server: SocketAddr,
    desired_slot: Option<u8>,
    assigned_slot: Option<u8>,
    name: String,
    buffer: [u8; MAX_PACKET_SIZE],
    last_join: Instant,
}

impl ControllerClient {
    fn connect(server: &str, desired_slot: Option<u8>, name: String) -> io::Result<Self> {
        let server_addr = resolve(server)?;
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        let mut client = Self {
            socket,
            server: server_addr,
            desired_slot,
            assigned_slot: None,
            name,
            buffer: [0; MAX_PACKET_SIZE],
            last_join: Instant::now() - RESEND_INTERVAL,
        };
        client.send_join()?;
        Ok(client)
    }

    fn is_ready(&self) -> bool {
        self.assigned_slot.is_some()
    }

    fn slot(&self) -> Option<u8> {
        self.assigned_slot
    }

    fn poll(&mut self) {
        while let Ok((len, addr)) = self.socket.recv_from(&mut self.buffer) {
            if addr != self.server {
                continue;
            }
            if let Ok(packet) = bincode::deserialize::<ServerPacket>(&self.buffer[..len]) {
                match packet {
                    ServerPacket::Ack { slot, .. } => {
                        self.assigned_slot = Some(slot);
                    }
                    ServerPacket::Reject { .. } => {
                        self.assigned_slot = None;
                    }
                }
            }
        }
        if !self.is_ready() && self.last_join.elapsed() >= RESEND_INTERVAL {
            let _ = self.send_join();
        }
    }

    fn send_join(&mut self) -> io::Result<()> {
        let packet = ClientPacket::Join {
            desired_slot: self.desired_slot,
            name: self.name.clone(),
        };
        let bytes = bincode::serialize(&packet).unwrap_or_default();
        self.socket.send_to(&bytes, self.server)?;
        self.last_join = Instant::now();
        Ok(())
    }

    fn send_state(&self, state: ControlState) {
        let Some(slot) = self.assigned_slot else {
            return;
        };
        let packet = ClientPacket::Input { slot, state };
        if let Ok(bytes) = bincode::serialize(&packet) {
            let _ = self.socket.send_to(&bytes, self.server);
        }
    }
}

fn resolve(addr: &str) -> io::Result<SocketAddr> {
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidInput, "invalid address"))
}

fn parse_args() -> (String, Option<u8>, String) {
    let mut args = std::env::args().skip(1);
    let mut server = String::from("127.0.0.1:6000");
    let mut slot = None;
    let mut name = whoami::username();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--server" => {
                if let Some(value) = args.next() {
                    server = value;
                }
            }
            "--slot" => {
                if let Some(value) = args.next() {
                    slot = value.parse::<u8>().ok();
                }
            }
            "--name" => {
                if let Some(value) = args.next() {
                    name = value;
                }
            }
            _ => {}
        }
    }
    (server, slot, name)
}

fn collect_input(active: bool) -> ControlState {
    if !active {
        return ControlState::idle();
    }
    let mut axis: i8 = 0;
    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        axis -= 1;
    }
    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
        axis += 1;
    }
    axis = axis.clamp(-1, 1);

    let mut state = ControlState {
        move_axis: axis,
        aim_up: is_key_down(KeyCode::Up),
        aim_down: is_key_down(KeyCode::Down),
        start_charge: is_key_pressed(KeyCode::Space),
        release_charge: is_key_released(KeyCode::Space),
        skill_command: None,
    };

    if is_key_pressed(KeyCode::Key1) {
        state.skill_command = Some(SkillCommand::Toggle(SkillKind::Jetpack));
    } else if is_key_pressed(KeyCode::Key2) {
        state.skill_command = Some(SkillCommand::Toggle(SkillKind::Heavy));
    } else if is_key_pressed(KeyCode::Key3) {
        state.skill_command = Some(SkillCommand::Toggle(SkillKind::Triple));
    } else if is_key_pressed(KeyCode::Key4) {
        state.skill_command = Some(SkillCommand::Toggle(SkillKind::Scatter));
    }

    state
}

#[macroquad::main("Remote Controller")]
async fn main() {
    let (server, slot, name) = parse_args();
    let mut client = match ControllerClient::connect(&server, slot, name.clone()) {
        Ok(client) => client,
        Err(err) => {
            eprintln!("Unable to create controller client: {err}");
            return;
        }
    };

    loop {
        client.poll();
        let ready = client.is_ready();
        let input_state = collect_input(ready);
        client.send_state(input_state);

        draw_interface(&client, ready, &server, &name);

        next_frame().await;
    }
}

fn draw_interface(client: &ControllerClient, ready: bool, server: &str, name: &str) {
    clear_background(BLACK);
    let status = if ready {
        format!("Connected as slot {}", client.slot().unwrap())
    } else {
        "Waiting for assignment".to_string()
    };
    draw_text(&format!("Controller for {name}"), 20.0, 40.0, 32.0, WHITE);
    draw_text(&format!("Server: {server}"), 20.0, 80.0, 24.0, LIGHTGRAY);
    draw_text(&status, 20.0, 120.0, 24.0, GREEN);
    draw_text(
        "Controls: A/D move, Up/Down aim, Space shoot, 1-4 toggle skills",
        20.0,
        180.0,
        20.0,
        WHITE,
    );
}

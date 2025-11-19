use macroquad::prelude::*;
use std::env;

use crate::config::{
    COLS, EXPLOSION_DAMAGE, EXPLOSION_RADIUS, MAP_PATH, PLAYER_COLORS, ROWS, TILE_SIZE,
    TURN_DURATION,
};
use crate::controls::{ControlState, SkillCommand};
use crate::map::Map;
use crate::net::host::NetworkInputHost;
use crate::player::{Player, apply_explosion_damage, create_players};
use crate::projectile::Projectile;
use crate::skills::build_shot_pattern;
use crate::ui::{
    draw_charge_bar, draw_game_over_scene, draw_skill_buttons, draw_turn_timer, skill_button_hit,
};

const LOCAL_PLAYER_SLOT: usize = 0;

pub struct LaunchOptions {
    pub listen_addr: Option<String>,
}

impl LaunchOptions {
    pub fn from_env() -> Self {
        let mut listen_addr = None;
        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--listen" => {
                    if let Some(addr) = args.next() {
                        listen_addr = Some(addr);
                    }
                }
                _ => {}
            }
        }
        Self { listen_addr }
    }
}

#[derive(Clone)]
pub enum GamePhase {
    StartMenu,
    PlayerSelect,
    Playing,
    GameOver { winner: Option<String> },
}

#[derive(Clone)]
struct ShotTask {
    time_until: f32,
    base_angle: f32,
    angle_offsets: Vec<f32>,
    power: f32,
    damage: f32,
    owner_idx: usize,
    origin: Vec2,
    facing: f32,
}

pub fn window_conf() -> Conf {
    Conf {
        window_title: "CSV Terrain".to_owned(),
        window_width: (COLS as f32 * TILE_SIZE) as i32,
        window_height: (ROWS as f32 * TILE_SIZE) as i32,
        ..Default::default()
    }
}

pub async fn run_game(options: LaunchOptions) {
    let mut map = Map::from_csv(MAP_PATH);
    let mut players: Vec<Player> = Vec::new();
    let mut active_index: usize = 0;
    let mut turn_timer = TURN_DURATION;
    let mut projectiles: Vec<Projectile> = Vec::new();
    let mut pending_shots: Vec<ShotTask> = Vec::new();
    let mut pending_turn_after_action = false;
    let mut phase = GamePhase::StartMenu;
    let mut net_host =
        options
            .listen_addr
            .as_ref()
            .and_then(|addr| match NetworkInputHost::bind(addr) {
                Ok(host) => Some(host),
                Err(err) => {
                    eprintln!("Failed to bind network listener at {addr}: {err}");
                    None
                }
            });

    'game: loop {
        let dt = get_frame_time();
        if let Some(server) = net_host.as_mut() {
            server.poll();
        }
        match phase.clone() {
            GamePhase::StartMenu => {
                clear_background(BLACK);
                let button_width = 220.0;
                let button_height = 80.0;
                let button_x = (screen_width() - button_width) * 0.5;
                let button_y = (screen_height() - button_height) * 0.5;
                let button_rect = Rect::new(button_x, button_y, button_width, button_height);
                let mouse = vec2(mouse_position().0, mouse_position().1);
                let hovered = button_rect.contains(mouse);
                let button_color = if hovered { DARKGRAY } else { GRAY };
                draw_rectangle(
                    button_rect.x,
                    button_rect.y,
                    button_rect.w,
                    button_rect.h,
                    button_color,
                );
                draw_rectangle_lines(
                    button_rect.x,
                    button_rect.y,
                    button_rect.w,
                    button_rect.h,
                    3.0,
                    WHITE,
                );

                let title = "Start";
                let title_dims = measure_text(title, None, 36, 1.0);
                draw_text(
                    title,
                    button_rect.x + (button_rect.w - title_dims.width) * 0.5,
                    button_rect.y + button_rect.h * 0.6,
                    36.0,
                    WHITE,
                );

                let hint = "Click or press Enter";
                let hint_dims = measure_text(hint, None, 20, 1.0);
                draw_text(
                    hint,
                    (screen_width() - hint_dims.width) * 0.5,
                    button_rect.y + button_rect.h + 40.0,
                    20.0,
                    LIGHTGRAY,
                );

                if (hovered && is_mouse_button_pressed(MouseButton::Left))
                    || is_key_pressed(KeyCode::Enter)
                {
                    phase = GamePhase::PlayerSelect;
                }
            }
            GamePhase::PlayerSelect => {
                clear_background(BLACK);
                map.draw();

                let title = "Select Player Count";
                let title_dims = measure_text(title, None, 32, 1.0);
                draw_text(
                    title,
                    (screen_width() - title_dims.width) * 0.5,
                    screen_height() * 0.25,
                    32.0,
                    WHITE,
                );

                let options = [1_usize, 2, 3, 4, 5];
                let button_width = 160.0;
                let button_height = 48.0;
                let gap = 16.0;
                let total_height =
                    options.len() as f32 * button_height + (options.len() as f32 - 1.0) * gap;
                let start_x = (screen_width() - button_width) * 0.5;
                let mut start_y = (screen_height() - total_height) * 0.5 + 40.0;

                let mouse = vec2(mouse_position().0, mouse_position().1);
                let mut chosen = None;

                for &count in &options {
                    let rect = Rect::new(start_x, start_y, button_width, button_height);
                    let hovered = rect.contains(mouse);
                    let color = if hovered { ORANGE } else { DARKGRAY };
                    draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
                    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, WHITE);

                    let label = format!("{count} Player{}", if count > 1 { "s" } else { "" });
                    let dims = measure_text(&label, None, 24, 1.0);
                    draw_text(
                        &label,
                        rect.x + (rect.w - dims.width) * 0.5,
                        rect.y + rect.h * 0.65,
                        24.0,
                        WHITE,
                    );

                    if hovered && is_mouse_button_pressed(MouseButton::Left) {
                        chosen = Some(count);
                    }

                    start_y += button_height + gap;
                }

                if let Some(count) = chosen {
                    map = Map::from_csv(MAP_PATH);
                    initialize_game(
                        &map,
                        count,
                        &mut players,
                        &mut active_index,
                        &mut turn_timer,
                        &mut projectiles,
                        &mut pending_shots,
                        &mut pending_turn_after_action,
                    );
                    if let Some(server) = net_host.as_mut() {
                        server.configure_slots(count);
                    }
                    phase = GamePhase::Playing;
                    continue 'game;
                }
            }
            GamePhase::Playing => {
                if players.is_empty() {
                    phase = GamePhase::GameOver { winner: None };
                    continue 'game;
                }

                if !ensure_active_player_alive(&mut active_index, players.as_slice()) {
                    phase = GamePhase::GameOver { winner: None };
                    continue 'game;
                }

                let controls_enabled = players[active_index].is_alive();

                let mut control_states = vec![ControlState::idle(); players.len()];
                if let Some(server) = net_host.as_ref() {
                    server.fill_states(&mut control_states);
                }

                if let Some(local_state) = control_states.get_mut(LOCAL_PLAYER_SLOT) {
                    let allow = controls_enabled && active_index == LOCAL_PLAYER_SLOT;
                    *local_state = collect_local_input(allow);
                }

                for idx in 0..players.len() {
                    let enable = idx == active_index && controls_enabled;
                    let input = control_states
                        .get(idx)
                        .copied()
                        .unwrap_or_else(ControlState::idle);
                    players[idx].update(&map, dt, enable, input);
                }

                if controls_enabled {
                    if let Some(cmd) = control_states
                        .get(active_index)
                        .and_then(|state| state.skill_command)
                    {
                        players[active_index].skills_mut().toggle(cmd.kind());
                    }
                }

                let release_charge = control_states
                    .get(active_index)
                    .map(|state| state.release_charge)
                    .unwrap_or(false);

                if controls_enabled && players[active_index].is_charging() && release_charge {
                    let charge = players[active_index].charge_power();
                    if charge > 0.0 {
                        if players[active_index].skills().jetpack_enabled() {
                            players[active_index].launch_self(charge);
                            pending_turn_after_action = true;
                        } else {
                            let base_angle = players[active_index].launch_angle();
                            let origin = players[active_index].shot_origin();
                            let facing = players[active_index].facing();
                            let (pattern, damage_multiplier) =
                                build_shot_pattern(players[active_index].skills());
                            let damage = EXPLOSION_DAMAGE * damage_multiplier;
                            if pattern.is_empty() {
                                pending_shots.push(ShotTask {
                                    time_until: 0.0,
                                    base_angle,
                                    angle_offsets: vec![0.0],
                                    power: charge,
                                    damage,
                                    owner_idx: active_index,
                                    origin,
                                    facing,
                                });
                            } else {
                                for (delay, offsets) in pattern {
                                    pending_shots.push(ShotTask {
                                        time_until: delay,
                                        base_angle,
                                        angle_offsets: offsets,
                                        power: charge,
                                        damage,
                                        owner_idx: active_index,
                                        origin,
                                        facing,
                                    });
                                }
                            }
                            pending_turn_after_action = true;
                        }
                    }
                    players[active_index].cancel_charge();
                }

                let mut ready_tasks = Vec::new();
                let mut idx = 0;
                while idx < pending_shots.len() {
                    pending_shots[idx].time_until -= dt;
                    if pending_shots[idx].time_until <= 0.0 {
                        ready_tasks.push(pending_shots.remove(idx));
                    } else {
                        idx += 1;
                    }
                }

                for task in ready_tasks {
                    let Some(player) = players.get(task.owner_idx) else {
                        continue;
                    };
                    let (min_angle, max_angle) = player.relative_launch_angle_bounds();
                    for offset in &task.angle_offsets {
                        let relative = (task.base_angle + *offset).clamp(min_angle, max_angle);
                        let world_angle = player.to_world_launch_angle(relative, task.facing);
                        projectiles.push(Projectile::launch(
                            task.origin,
                            task.facing,
                            world_angle,
                            task.power,
                            task.damage,
                        ));
                    }
                }

                let mut impacts: Vec<(Vec2, f32)> = Vec::new();
                for projectile in projectiles.iter_mut() {
                    if !projectile.is_active() {
                        continue;
                    }
                    if let Some(hit_pos) = projectile.update(&map, dt) {
                        impacts.push((hit_pos, projectile.damage()));
                        projectile.deactivate();
                    }
                }

                for (hit_pos, damage) in impacts {
                    map.carve_circle(hit_pos, EXPLOSION_RADIUS);
                    apply_explosion_damage(hit_pos, damage, &mut players);
                }

                projectiles.retain(|p| p.is_active());

                let alive_count = players.iter().filter(|p| p.is_alive()).count();
                if players.len() > 1 && alive_count <= 1 {
                    let winner = players
                        .iter()
                        .find(|p| p.is_alive())
                        .map(|p| p.name().to_string());
                    pending_shots.clear();
                    projectiles.clear();
                    pending_turn_after_action = false;
                    phase = GamePhase::GameOver { winner };
                    continue 'game;
                }

                if players[active_index].is_alive() && !pending_turn_after_action {
                    turn_timer -= dt;
                }

                let mut turn_should_end = false;
                if !players[active_index].is_alive() {
                    turn_should_end = true;
                    pending_turn_after_action = false;
                    pending_shots.clear();
                    projectiles.clear();
                }

                if pending_turn_after_action && pending_shots.is_empty() && projectiles.is_empty() {
                    turn_should_end = true;
                    pending_turn_after_action = false;
                }

                if turn_timer <= 0.0 {
                    turn_should_end = true;
                    pending_turn_after_action = false;
                    pending_shots.clear();
                }

                if turn_should_end {
                    advance_turn(&mut active_index, &mut turn_timer, &mut players);
                }

                clear_background(BLACK);
                map.draw();
                for idx in 0..players.len() {
                    let is_active = idx == active_index && players[idx].is_alive();
                    players[idx].draw(is_active);
                }
                for projectile in &projectiles {
                    projectile.draw();
                }
                let active_player = players.get(active_index).filter(|p| p.is_alive());
                draw_charge_bar(active_player);
                draw_skill_buttons(players.get(active_index));
                draw_turn_timer(turn_timer, active_player.map(|p| p.name()));
            }
            GamePhase::GameOver { ref winner } => {
                clear_background(BLACK);
                draw_game_over_scene(&map, &players, winner.as_deref());
                if is_key_pressed(KeyCode::Enter) || is_mouse_button_pressed(MouseButton::Left) {
                    phase = GamePhase::StartMenu;
                    players.clear();
                    projectiles.clear();
                    pending_shots.clear();
                    pending_turn_after_action = false;
                    turn_timer = TURN_DURATION;
                }
            }
        }
        next_frame().await;
    }
}

fn collect_local_input(allow: bool) -> ControlState {
    if !allow {
        return ControlState::idle();
    }
    let mut move_axis: i8 = 0;
    if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
        move_axis -= 1;
    }
    if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
        move_axis += 1;
    }
    move_axis = move_axis.clamp(-1, 1);

    let mut state = ControlState {
        move_axis,
        aim_up: is_key_down(KeyCode::Up),
        aim_down: is_key_down(KeyCode::Down),
        start_charge: is_key_pressed(KeyCode::Space),
        release_charge: is_key_released(KeyCode::Space),
        skill_command: None,
    };

    if is_mouse_button_pressed(MouseButton::Left) {
        let mouse = vec2(mouse_position().0, mouse_position().1);
        if let Some(skill) = skill_button_hit(mouse) {
            state.skill_command = Some(SkillCommand::Toggle(skill));
        }
    }

    state
}

fn initialize_game(
    map: &Map,
    player_count: usize,
    players: &mut Vec<Player>,
    active_index: &mut usize,
    turn_timer: &mut f32,
    projectiles: &mut Vec<Projectile>,
    pending_shots: &mut Vec<ShotTask>,
    pending_turn_after_action: &mut bool,
) {
    *players = create_players(map, player_count, &PLAYER_COLORS);
    for player in players.iter_mut() {
        player.refresh_surface_angle(map);
    }
    *active_index = 0;
    let _ = ensure_active_player_alive(active_index, players.as_slice());
    *turn_timer = TURN_DURATION;
    projectiles.clear();
    pending_shots.clear();
    *pending_turn_after_action = false;
}

fn ensure_active_player_alive(active_index: &mut usize, players: &[Player]) -> bool {
    if players.iter().all(|p| !p.is_alive()) {
        return false;
    }
    if players[*active_index].is_alive() {
        return true;
    }
    let len = players.len();
    for step in 1..=len {
        let idx = (*active_index + step) % len;
        if players[idx].is_alive() {
            *active_index = idx;
            return true;
        }
    }
    false
}

fn advance_turn(active_index: &mut usize, turn_timer: &mut f32, players: &mut [Player]) {
    if let Some(active) = players.get_mut(*active_index) {
        active.cancel_charge();
    }
    *turn_timer = TURN_DURATION;

    if players.iter().all(|p| !p.is_alive()) {
        return;
    }

    let len = players.len();
    for step in 1..=len {
        let idx = (*active_index + step) % len;
        if players[idx].is_alive() {
            *active_index = idx;
            break;
        }
    }
}

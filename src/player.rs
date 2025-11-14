use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::f32::consts::PI;

use crate::config::{
    EXPLOSION_RADIUS, GROUND_EPSILON, HEALTH_BAR_HEIGHT, HEALTH_BAR_WIDTH, MAX_DOWN_STEP,
    MAX_HEALTH, MAX_LAUNCH_ANGLE, MAX_UP_STEP, MIN_LAUNCH_ANGLE, MOVE_SPEED, PLAYER_HEIGHT,
    PLAYER_NAMES, PLAYER_SUBSTEP_DT, PLAYER_WIDTH, POWER_MAX, POWER_RATE, SCATTER_OFFSET_RAD,
    SMOOTH_FACTOR, TILE_SIZE,
};
use crate::map::Map;
use crate::skills::SkillLoadout;

pub struct Player {
    pos: Vec2,
    velocity: Vec2,
    on_ground: bool,
    facing: f32,
    launch_angle: f32,
    is_charging: bool,
    charge_power: f32,
    is_airpack_flight: bool,
    health: f32,
    color: Color,
    name: &'static str,
    skills: SkillLoadout,
    surface_angle: f32,
}

impl Player {
    pub fn update(&mut self, map: &Map, dt: f32, controls_enabled: bool) {
        if !self.is_alive() {
            self.velocity = Vec2::ZERO;
            self.is_airpack_flight = false;
            self.is_charging = false;
            let player_top = self.pos.y - PLAYER_HEIGHT;
            let player_left = self.pos.x - PLAYER_WIDTH * 0.5;
            let player_right = self.pos.x + PLAYER_WIDTH * 0.5;
            if let Some(floor_y) = map.find_floor_below(player_left, player_right, player_top) {
                self.pos.y = floor_y;
            }
            return;
        }

        if self.is_airpack_flight {
            // retain velocity while jetpacking
        } else if controls_enabled && !self.is_charging {
            let mut input: f32 = 0.0;
            if is_key_down(KeyCode::A) || is_key_down(KeyCode::Left) {
                input -= 1.0;
            }
            if is_key_down(KeyCode::D) || is_key_down(KeyCode::Right) {
                input += 1.0;
            }
            if input.abs() > 0.1 {
                self.facing = input.signum();
            }
            let target_speed = input * MOVE_SPEED;
            let smoothing = (SMOOTH_FACTOR * dt).clamp(0.0, 1.0);
            self.velocity.x += (target_speed - self.velocity.x) * smoothing;
            if target_speed.abs() < 1.0 && self.velocity.x.abs() < 1.0 {
                self.velocity.x = 0.0;
            }
        } else {
            self.velocity.x = 0.0;
        }

        let mut horizontal_step = self.velocity.x * dt;
        let player_top = self.pos.y - PLAYER_HEIGHT;
        let player_left = self.pos.x - PLAYER_WIDTH * 0.5;
        let player_right = self.pos.x + PLAYER_WIDTH * 0.5;
        let current_floor = map.find_floor_below(player_left, player_right, player_top);

        if self.on_ground && horizontal_step.abs() > f32::EPSILON {
            let target_left = (self.pos.x + horizontal_step) - PLAYER_WIDTH * 0.5;
            let target_right = (self.pos.x + horizontal_step) + PLAYER_WIDTH * 0.5;
            let target_floor = map.find_floor_below(target_left, target_right, player_top);
            if let Some(current_surface) = current_floor {
                if let Some(target_surface) = target_floor {
                    if target_surface < current_surface
                        && (current_surface - target_surface) > MAX_UP_STEP
                    {
                        horizontal_step = 0.0;
                        self.velocity.x = 0.0;
                    } else if target_surface > current_surface
                        && (target_surface - current_surface) > MAX_DOWN_STEP
                    {
                        horizontal_step = 0.0;
                        self.velocity.x = 0.0;
                    }
                } else {
                    horizontal_step = 0.0;
                    self.velocity.x = 0.0;
                }
            }
        }

        let min_x = TILE_SIZE * 0.5;
        let max_x = map.max_x();
        self.pos.x = (self.pos.x + horizontal_step).clamp(min_x, max_x);
        if self.pos.x <= min_x + f32::EPSILON || self.pos.x >= max_x - f32::EPSILON {
            self.velocity.x = 0.0;
        }

        if controls_enabled && is_key_down(KeyCode::Up) {
            self.launch_angle += 1.2 * dt;
        }
        if controls_enabled && is_key_down(KeyCode::Down) {
            self.launch_angle -= 1.2 * dt;
        }
        let (min_angle, max_angle) = self.relative_launch_angle_bounds();
        self.launch_angle = self.launch_angle.clamp(min_angle, max_angle);

        if controls_enabled && is_key_pressed(KeyCode::Space) {
            self.is_charging = true;
            self.charge_power = 0.0;
        } else if !controls_enabled {
            self.is_charging = false;
            self.charge_power = 0.0;
        }
        if self.is_charging {
            self.charge_power = (self.charge_power + POWER_RATE * dt).min(POWER_MAX);
        }

        self.on_ground = false;
        let mut remaining_time = dt;
        while remaining_time > f32::EPSILON {
            let step_dt = remaining_time.min(PLAYER_SUBSTEP_DT);
            remaining_time -= step_dt;

            self.velocity.y += crate::config::GRAVITY * step_dt;
            let proposed_y = self.pos.y + self.velocity.y * step_dt;
            let next_top = proposed_y - PLAYER_HEIGHT;
            let next_left = self.pos.x - PLAYER_WIDTH * 0.5;
            let next_right = self.pos.x + PLAYER_WIDTH * 0.5;

            let mut landed = false;
            if self.velocity.y >= 0.0 {
                if let Some(floor_y) = map.find_floor_below(next_left, next_right, next_top) {
                    if proposed_y >= floor_y - GROUND_EPSILON {
                        self.pos.y = floor_y;
                        self.velocity.y = 0.0;
                        self.on_ground = true;
                        self.is_airpack_flight = false;
                        landed = true;
                    }
                }
            }
            if landed {
                break;
            }
            self.pos.y = proposed_y;
        }

        if !self.on_ground {
            let next_top = self.pos.y - PLAYER_HEIGHT;
            let next_left = self.pos.x - PLAYER_WIDTH * 0.5;
            let next_right = self.pos.x + PLAYER_WIDTH * 0.5;

            if let Some(floor_y) = map.find_floor_below(next_left, next_right, next_top) {
                if self.velocity.y >= 0.0 && self.pos.y >= floor_y - GROUND_EPSILON {
                    self.pos.y = floor_y;
                    self.velocity.y = 0.0;
                    self.on_ground = true;
                    self.is_airpack_flight = false;
                }
            } else if self.pos.y - PLAYER_HEIGHT > map.total_height() + PLAYER_HEIGHT {
                self.respawn_top_dead(map);
                return;
            }
        }

        if self.pos.y < -PLAYER_HEIGHT * 2.0 {
            self.pos.y = -PLAYER_HEIGHT * 2.0;
            if self.velocity.y < 0.0 {
                self.velocity.y = 0.0;
            }
        }

        if self.on_ground {
            self.refresh_surface_angle(map);
        }
    }

    pub fn draw(&self, active: bool) {
        let alive = self.is_alive();
        let body_top = if alive {
            self.pos.y - PLAYER_HEIGHT
        } else {
            self.pos.y - PLAYER_WIDTH
        };
        let body_color = if alive {
            if active {
                self.color
            } else {
                Color::new(
                    self.color.r * 0.7,
                    self.color.g * 0.7,
                    self.color.b * 0.7,
                    self.color.a,
                )
            }
        } else {
            DARKGRAY
        };

        if alive {
            let foot = vec2(self.pos.x, self.pos.y);
            let half_width = PLAYER_WIDTH * 0.5;
            let height = PLAYER_HEIGHT;
            let tangent_angle = self.surface_angle;
            let tangent_dir = vec2(tangent_angle.cos(), -tangent_angle.sin());
            let normal_dir = vec2(tangent_dir.y, -tangent_dir.x);

            let bottom_left = foot - tangent_dir * half_width;
            let bottom_right = foot + tangent_dir * half_width;
            let top_left = bottom_left + normal_dir * height;
            let top_right = bottom_right + normal_dir * height;

            draw_triangle(bottom_left, top_left, top_right, body_color);
            draw_triangle(bottom_left, top_right, bottom_right, body_color);

            if active {
                draw_line(
                    bottom_left.x,
                    bottom_left.y,
                    bottom_right.x,
                    bottom_right.y,
                    2.0,
                    WHITE,
                );
                draw_line(
                    bottom_right.x,
                    bottom_right.y,
                    top_right.x,
                    top_right.y,
                    2.0,
                    WHITE,
                );
                draw_line(top_right.x, top_right.y, top_left.x, top_left.y, 2.0, WHITE);
                draw_line(
                    top_left.x,
                    top_left.y,
                    bottom_left.x,
                    bottom_left.y,
                    2.0,
                    WHITE,
                );
            }

            let aim_origin = foot + normal_dir * (-PLAYER_HEIGHT * 0.5);
            let aim_dir = self.shot_direction();
            let aim_end = aim_origin + aim_dir * 40.0;
            draw_line(aim_origin.x, aim_origin.y, aim_end.x, aim_end.y, 2.0, WHITE);
        } else {
            let body_width = PLAYER_HEIGHT;
            let body_height = PLAYER_WIDTH;
            let x = self.pos.x - body_width * 0.5;
            draw_rectangle(x, body_top, body_width, body_height, body_color);
        }

        let bar_x = self.pos.x - HEALTH_BAR_WIDTH * 0.5;
        let bar_y = body_top - HEALTH_BAR_HEIGHT - 6.0;
        draw_rectangle(bar_x, bar_y, HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT, DARKGRAY);
        let health_ratio = self.health_ratio();
        if health_ratio > 0.0 {
            draw_rectangle(
                bar_x,
                bar_y,
                HEALTH_BAR_WIDTH * health_ratio,
                HEALTH_BAR_HEIGHT,
                GREEN,
            );
        }
        draw_rectangle_lines(
            bar_x,
            bar_y,
            HEALTH_BAR_WIDTH,
            HEALTH_BAR_HEIGHT,
            1.0,
            WHITE,
        );

        let percent_text = format!("{:.0}%", (health_ratio * 100.0).round());
        let percent_dims = measure_text(&percent_text, None, 16, 1.0);
        draw_text(
            &percent_text,
            self.pos.x - percent_dims.width * 0.5,
            bar_y - 4.0,
            16.0,
            WHITE,
        );

        let mut text_cursor = bar_y - 22.0;
        let labels = self.skills.labels();
        if !labels.is_empty() {
            let skill_line = labels.join(" & ");
            let dims = measure_text(&skill_line, None, 16, 1.0);
            draw_text(
                &skill_line,
                self.pos.x - dims.width * 0.5,
                text_cursor,
                16.0,
                WHITE,
            );
            text_cursor -= 18.0;
        }

        let name_dims = measure_text(self.name, None, 16, 1.0);
        draw_text(
            self.name,
            self.pos.x - name_dims.width * 0.5,
            text_cursor,
            16.0,
            WHITE,
        );
    }

    pub fn launch_self(&mut self, power: f32) {
        let mut dir = self.shot_direction();
        if dir.length_squared() <= f32::EPSILON {
            dir = vec2(self.facing, 0.0);
        }
        self.is_airpack_flight = true;
        self.on_ground = false;
        self.velocity = dir * power;
        self.pos += dir * 2.0;
    }

    pub fn apply_damage(&mut self, amount: f32) {
        let previous = self.health;
        self.health = (self.health - amount).max(0.0);
        if previous > 0.0 && self.health <= 0.0 {
            self.cancel_charge();
            self.velocity = Vec2::ZERO;
            self.is_airpack_flight = false;
        }
    }

    pub fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    pub fn cancel_charge(&mut self) {
        self.is_charging = false;
        self.charge_power = 0.0;
    }

    pub fn refresh_surface_angle(&mut self, map: &Map) {
        let half_width = PLAYER_WIDTH * 0.5;
        let left_x = (self.pos.x - half_width).clamp(0.0, map.total_width());
        let right_x = (self.pos.x + half_width).clamp(0.0, map.total_width());
        let sample_width = (right_x - left_x).max(TILE_SIZE);
        let probe_top = self.pos.y - PLAYER_HEIGHT;

        let span = TILE_SIZE * 0.25;
        let left_end = (left_x + span).min(map.total_width());
        let right_start = if right_x >= span { right_x - span } else { 0.0 };

        let left_floor = map
            .find_floor_below(left_x, left_end.max(left_x), probe_top)
            .unwrap_or(self.pos.y);
        let right_floor = map
            .find_floor_below(right_start.min(right_x), right_x, probe_top)
            .unwrap_or(self.pos.y);

        let dy = right_floor - left_floor;
        let angle = (-dy).atan2(sample_width);
        self.surface_angle = angle.clamp(-PI / 4.0, PI / 4.0);
    }

    pub fn relative_launch_angle_bounds(&self) -> (f32, f32) {
        if self.skills.scatter_selected() {
            (
                (MIN_LAUNCH_ANGLE - SCATTER_OFFSET_RAD).max(0.0),
                MAX_LAUNCH_ANGLE + SCATTER_OFFSET_RAD,
            )
        } else {
            (MIN_LAUNCH_ANGLE, MAX_LAUNCH_ANGLE)
        }
    }

    pub fn to_world_launch_angle(&self, relative_angle: f32, facing: f32) -> f32 {
        let world = if facing >= 0.0 {
            relative_angle + self.surface_angle
        } else {
            relative_angle - self.surface_angle
        };
        world.clamp(0.01, PI - 0.01)
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn charge_power(&self) -> f32 {
        self.charge_power
    }

    pub fn is_charging(&self) -> bool {
        self.is_charging
    }

    pub fn facing(&self) -> f32 {
        self.facing
    }

    pub fn launch_angle(&self) -> f32 {
        self.launch_angle
    }

    pub fn skills(&self) -> &SkillLoadout {
        &self.skills
    }

    pub fn skills_mut(&mut self) -> &mut SkillLoadout {
        &mut self.skills
    }

    pub fn shot_origin(&self) -> Vec2 {
        vec2(self.pos.x, self.pos.y - PLAYER_HEIGHT * 0.5)
    }

    pub fn health_ratio(&self) -> f32 {
        (self.health / MAX_HEALTH).clamp(0.0, 1.0)
    }

    fn shot_direction(&self) -> Vec2 {
        let (min_angle, max_angle) = self.relative_launch_angle_bounds();
        let clamped = self.launch_angle.clamp(min_angle, max_angle);
        let world_angle = self.to_world_launch_angle(clamped, self.facing);
        let mut dir = vec2(self.facing * world_angle.cos(), -world_angle.sin());
        if dir.length_squared() > f32::EPSILON {
            dir = dir.normalize();
        } else {
            dir = vec2(self.facing, 0.0);
        }
        dir
    }

    fn respawn_top_dead(&mut self, map: &Map) {
        let min_x = TILE_SIZE * 0.5;
        let max_x = map.max_x();
        let spawn_x = gen_range(min_x, max_x);
        self.pos = vec2(spawn_x, PLAYER_HEIGHT);
        self.velocity = Vec2::ZERO;
        self.on_ground = false;
        self.is_airpack_flight = false;
        self.is_charging = false;
        self.health = 0.0;
        self.surface_angle = 0.0;
        self.launch_angle = MIN_LAUNCH_ANGLE;
    }
}

pub fn apply_explosion_damage(center: Vec2, base_damage: f32, players: &mut [Player]) {
    for player in players.iter_mut() {
        if !player.is_alive() {
            continue;
        }
        let torso_center = vec2(player.pos.x, player.pos.y - PLAYER_HEIGHT * 0.5);
        let distance = torso_center.distance(center);
        let effective_radius = EXPLOSION_RADIUS + PLAYER_WIDTH * 0.5;
        if distance <= effective_radius {
            let falloff = 1.0 - (distance / effective_radius).clamp(0.0, 1.0);
            let damage = base_damage * falloff;
            if damage > 0.0 {
                player.apply_damage(damage);
            }
        }
    }
}

pub fn create_players(map: &Map, count: usize, colors: &[Color]) -> Vec<Player> {
    let mut players = Vec::with_capacity(count);
    if count == 0 {
        return players;
    }

    let width = map.total_width();
    let spacing = width / (count as f32 + 1.0);
    for idx in 0..count {
        let hint = spacing * (idx as f32 + 1.0);
        let color = colors.get(idx % colors.len()).copied().unwrap_or(WHITE);
        let name = PLAYER_NAMES
            .get(idx % PLAYER_NAMES.len())
            .copied()
            .unwrap_or("Player");
        players.push(Player::spawn_with_hint(map, idx, color, hint, name));
    }

    players
}

impl Player {
    fn spawn_with_hint(
        map: &Map,
        id: usize,
        color: Color,
        preferred_x: f32,
        name: &'static str,
    ) -> Self {
        let min_x = TILE_SIZE * 0.5;
        let max_x = map.max_x();
        let mut spawn_x = preferred_x.clamp(min_x, max_x);
        let mut found = false;
        for _ in 0..256 {
            let left = spawn_x - PLAYER_WIDTH * 0.5;
            let right = spawn_x + PLAYER_WIDTH * 0.5;
            if map.find_floor_below(left, right, -PLAYER_HEIGHT).is_some() {
                found = true;
                break;
            }
            spawn_x = gen_range(min_x, max_x);
        }
        if !found {
            spawn_x = (min_x + max_x) * 0.5;
        }

        Player {
            pos: vec2(spawn_x, PLAYER_HEIGHT),
            velocity: vec2(0.0, 0.0),
            on_ground: false,
            facing: if id % 2 == 0 { 1.0 } else { -1.0 },
            launch_angle: MIN_LAUNCH_ANGLE,
            is_charging: false,
            charge_power: 0.0,
            is_airpack_flight: false,
            health: MAX_HEALTH,
            color,
            name,
            skills: SkillLoadout::new(),
            surface_angle: 0.0,
        }
    }
}

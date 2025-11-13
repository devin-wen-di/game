use macroquad::prelude::*;
use macroquad::rand::gen_range;

const MAP_PATH: &str = "src/map.csv";
const TILE_SIZE: f32 = 2.0;
const COLS: usize = 768;
const ROWS: usize = 512;
const PLAYER_WIDTH: f32 = TILE_SIZE * 4.0;
const PLAYER_HEIGHT: f32 = TILE_SIZE * 8.0;
const MOVE_SPEED: f32 = 60.0;
const SMOOTH_FACTOR: f32 = 12.0;
const GRAVITY: f32 = 400.0;
const MAX_UP_STEP: f32 = TILE_SIZE * 3.0;
const MAX_DOWN_STEP: f32 = TILE_SIZE * 6.0;
const PLAYER_SUBSTEP_DT: f32 = 1.0 / 120.0;
const PROJECTILE_SUBSTEP_DT: f32 = 1.0 / 240.0;
const GROUND_EPSILON: f32 = 0.25;
const MIN_LAUNCH_ANGLE: f32 = 30.0_f32.to_radians();
const MAX_LAUNCH_ANGLE: f32 = 80.0_f32.to_radians();
const POWER_MAX: f32 = 1000.0;
const POWER_RATE: f32 = 240.0;
const PROJECTILE_RADIUS: f32 = 6.0;
const EXPLOSION_RADIUS: f32 = 50.0;
const EXPLOSION_DAMAGE: f32 = 35.0;
const CHARGE_BAR_WIDTH: f32 = 220.0;
const CHARGE_BAR_HEIGHT: f32 = 12.0;
const CHARGE_BAR_MARGIN: f32 = 16.0;
const AIRPACK_BTN_WIDTH: f32 = 140.0;
const AIRPACK_BTN_HEIGHT: f32 = 40.0;
const AIRPACK_BTN_MARGIN: f32 = 16.0;
const TURN_DURATION: f32 = 20.0;
const MAX_HEALTH: f32 = 100.0;
const HEALTH_BAR_WIDTH: f32 = PLAYER_WIDTH;
const HEALTH_BAR_HEIGHT: f32 = 6.0;

fn window_conf() -> Conf {
    Conf {
        window_title: "CSV Terrain".to_owned(),
        window_width: (COLS as f32 * TILE_SIZE) as i32,
        window_height: (ROWS as f32 * TILE_SIZE) as i32,
        ..Default::default()
    }
}

#[derive(Clone)]
struct Map {
    tiles: Vec<Vec<u8>>,
    rows: usize,
    cols: usize,
}

impl Map {
    fn from_csv(path: &str) -> Self {
        let content = std::fs::read_to_string(path)
            .unwrap_or_else(|err| panic!("读取 {path} 失败: {err}"));
        let tiles: Vec<Vec<u8>> = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.split(',')
                    .map(|v| v.trim().parse::<u8>().unwrap_or(0))
                    .collect()
            })
            .collect();

        let rows = tiles.len();
        let cols = tiles.first().map_or(0, Vec::len);

        Self { tiles, rows, cols }
    }

    fn total_width(&self) -> f32 {
        self.cols as f32 * TILE_SIZE
    }

    fn total_height(&self) -> f32 {
        self.rows as f32 * TILE_SIZE
    }

    fn is_solid(&self, row: usize, col: usize) -> bool {
        self.tiles
            .get(row)
            .and_then(|r| r.get(col))
            .map(|v| *v != 0)
            .unwrap_or(false)
    }

    fn max_x(&self) -> f32 {
        (self.cols as f32 - 0.5) * TILE_SIZE
    }

    fn draw(&self) {
        for (row_idx, row) in self.tiles.iter().enumerate() {
            for (col_idx, value) in row.iter().enumerate() {
                let color = match value {
                    1 => GREEN,
                    2 => LIGHTGRAY,
                    _ => continue,
                };

                let x = col_idx as f32 * TILE_SIZE;
                let y = (self.rows - 1 - row_idx) as f32 * TILE_SIZE;
                draw_rectangle(x, y, TILE_SIZE, TILE_SIZE, color);
            }
        }
    }

    fn find_floor_below(&self, left: f32, right: f32, from_y: f32) -> Option<f32> {
        if self.rows == 0 || self.cols == 0 {
            return None;
        }

        let total_width = self.total_width();
        if right <= 0.0 || left >= total_width {
            return None;
        }

        let clamped_left = left.max(0.0);
        let clamped_right = right.min(total_width - f32::EPSILON);
        if clamped_left > clamped_right {
            return None;
        }

        let mut start_col = (clamped_left / TILE_SIZE).floor() as isize;
        let mut end_col = (clamped_right / TILE_SIZE).floor() as isize;
        let max_col = self.cols as isize - 1;
        start_col = start_col.clamp(0, max_col);
        end_col = end_col.clamp(0, max_col);
        if start_col > end_col {
            return None;
        }

        if from_y >= self.total_height() {
            return None;
        }

        let mut screen_row = if from_y < 0.0 {
            0
        } else {
            (from_y / TILE_SIZE).floor() as usize
        };
        if screen_row >= self.rows {
            screen_row = self.rows - 1;
        }

        for screen_row_idx in screen_row..self.rows {
            let row = self.rows - 1 - screen_row_idx;
            let tile_top = screen_row_idx as f32 * TILE_SIZE;

            for col in start_col as usize..=end_col as usize {
                if self.is_solid(row, col) {
                    return Some(tile_top);
                }
            }
        }

        None
    }

    fn world_to_grid(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let col = (x / TILE_SIZE).floor() as isize;
        let screen_row = (y / TILE_SIZE).floor() as isize;
        if col < 0 || screen_row < 0 || col as usize >= self.cols || screen_row as usize >= self.rows {
            None
        } else {
            let row = self.rows as isize - 1 - screen_row;
            Some((row as usize, col as usize))
        }
    }

    fn carve_circle(&mut self, center: Vec2, radius: f32) {
        if radius <= 0.0 {
            return;
        }

        let min_col = ((center.x - radius) / TILE_SIZE).floor() as isize;
        let max_col = ((center.x + radius) / TILE_SIZE).ceil() as isize;
        let min_screen_row = ((center.y - radius) / TILE_SIZE).floor() as isize;
        let max_screen_row = ((center.y + radius) / TILE_SIZE).ceil() as isize;

        for screen_row in min_screen_row..=max_screen_row {
            if screen_row < 0 || screen_row as usize >= self.rows {
                continue;
            }
            let row = self.rows as isize - 1 - screen_row;
            for col in min_col..=max_col {
                if col < 0 || col as usize >= self.cols {
                    continue;
                }
                let tile_center = vec2(
                    (col as f32 + 0.5) * TILE_SIZE,
                    (screen_row as f32 + 0.5) * TILE_SIZE,
                );
                if tile_center.distance(center) <= radius {
                    if let Some(tile_row) = self.tiles.get_mut(row as usize) {
                        if let Some(tile) = tile_row.get_mut(col as usize) {
                            *tile = 0;
                        }
                    }
                }
            }
        }
    }
}

struct Player {
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
    id: usize,
    spawn_hint: f32,
    name: &'static str,
}

impl Player {
    fn spawn_with_hint(map: &Map, id: usize, color: Color, preferred_x: f32, name: &'static str) -> Self {
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

        Self {
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
            id,
            spawn_hint: preferred_x,
            name,
        }
    }

    fn reset(&mut self, map: &Map) {
        let snapshot = Player::spawn_with_hint(map, self.id, self.color, self.spawn_hint, self.name);
        let preserved_health = self.health;
        *self = snapshot;
        self.health = preserved_health;
    }

    fn update(&mut self, map: &Map, dt: f32, controls_enabled: bool) {
        if self.is_airpack_flight {
            // preserve launch velocity while airborne due to airpack skill
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

        let mut step = self.velocity.x * dt;
        let player_top = self.pos.y - PLAYER_HEIGHT;
        let player_left = self.pos.x - PLAYER_WIDTH * 0.5;
        let player_right = self.pos.x + PLAYER_WIDTH * 0.5;
        let current_floor = map.find_floor_below(player_left, player_right, player_top);

        if self.on_ground && step.abs() > f32::EPSILON {
            let target_left = (self.pos.x + step) - PLAYER_WIDTH * 0.5;
            let target_right = (self.pos.x + step) + PLAYER_WIDTH * 0.5;
            let target_floor = map.find_floor_below(target_left, target_right, player_top);
            if let Some(current_surface) = current_floor {
                if let Some(target_surface) = target_floor {
                    if target_surface < current_surface
                        && (current_surface - target_surface) > MAX_UP_STEP
                    {
                        step = 0.0;
                        self.velocity.x = 0.0;
                    } else if target_surface > current_surface
                        && (target_surface - current_surface) > MAX_DOWN_STEP
                    {
                        step = 0.0;
                        self.velocity.x = 0.0;
                    }
                } else {
                    step = 0.0;
                    self.velocity.x = 0.0;
                }
            }
        }

        let min_x = TILE_SIZE * 0.5;
        let max_x = map.max_x();
        self.pos.x = (self.pos.x + step).clamp(min_x, max_x);
        if self.pos.x <= min_x + f32::EPSILON || self.pos.x >= max_x - f32::EPSILON {
            self.velocity.x = 0.0;
        }

        if controls_enabled && is_key_down(KeyCode::Up) {
            self.launch_angle = (self.launch_angle + 1.2 * dt).min(MAX_LAUNCH_ANGLE);
        }
        if controls_enabled && is_key_down(KeyCode::Down) {
            self.launch_angle = (self.launch_angle - 1.2 * dt).max(MIN_LAUNCH_ANGLE);
        }

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

            self.velocity.y += GRAVITY * step_dt;
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
                self.reset(map);
                return;
            }
        }

        if self.pos.y < -PLAYER_HEIGHT * 2.0 {
            self.pos.y = -PLAYER_HEIGHT * 2.0;
            if self.velocity.y < 0.0 {
                self.velocity.y = 0.0;
            }
        }
    }

    fn draw(&self, active: bool) {
        let x = self.pos.x - PLAYER_WIDTH * 0.5;
        let y = self.pos.y - PLAYER_HEIGHT;
        let body_color = if active {
            self.color
        } else {
            Color::new(
                self.color.r * 0.7,
                self.color.g * 0.7,
                self.color.b * 0.7,
                self.color.a,
            )
        };
        draw_rectangle(x, y, PLAYER_WIDTH, PLAYER_HEIGHT, body_color);
        if active {
            draw_rectangle_lines(x - 2.0, y - 2.0, PLAYER_WIDTH + 4.0, PLAYER_HEIGHT + 4.0, 2.0, WHITE);
        }

        let aim_origin = vec2(self.pos.x, self.pos.y - PLAYER_HEIGHT * 0.5);
        let direction = vec2(self.facing * self.launch_angle.cos(), -self.launch_angle.sin());
        let aim_end = aim_origin + direction.normalize() * 40.0;
        draw_line(aim_origin.x, aim_origin.y, aim_end.x, aim_end.y, 2.0, WHITE);

        let bar_x = self.pos.x - HEALTH_BAR_WIDTH * 0.5;
        let bar_y = y - HEALTH_BAR_HEIGHT - 6.0;
        draw_rectangle(bar_x, bar_y, HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT, DARKGRAY);
        let health_ratio = (self.health / MAX_HEALTH).clamp(0.0, 1.0);
        if health_ratio > 0.0 {
            draw_rectangle(bar_x, bar_y, HEALTH_BAR_WIDTH * health_ratio, HEALTH_BAR_HEIGHT, GREEN);
        }
        draw_rectangle_lines(bar_x, bar_y, HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT, 1.0, WHITE);

        let name_dims = measure_text(self.name, None, 16, 1.0);
        draw_text(
            self.name,
            self.pos.x - name_dims.width * 0.5,
            bar_y - 4.0,
            16.0,
            WHITE,
        );
    }

    fn launch_self(&mut self, power: f32) {
        let mut dir = vec2(self.facing * self.launch_angle.cos(), -self.launch_angle.sin());
        if dir.length_squared() > f32::EPSILON {
            dir = dir.normalize();
        } else {
            dir = vec2(self.facing, 0.0);
        }
        self.is_airpack_flight = true;
        self.on_ground = false;
        self.velocity = dir * power;
        self.pos += dir * 2.0;
    }

    fn apply_damage(&mut self, amount: f32) {
        self.health = (self.health - amount).max(0.0);
    }

    fn is_alive(&self) -> bool {
        self.health > 0.0
    }

    fn cancel_charge(&mut self) {
        self.is_charging = false;
        self.charge_power = 0.0;
    }
}

struct Projectile {
    pos: Vec2,
    velocity: Vec2,
    active: bool,
    owner_id: usize,
}

impl Projectile {
    fn launch(origin: Vec2, facing: f32, angle: f32, power: f32, owner_id: usize) -> Self {
        let dir = vec2(facing * angle.cos(), -angle.sin()).normalize();
        Self {
            pos: origin,
            velocity: dir * power,
            active: true,
            owner_id,
        }
    }

    fn update(&mut self, map: &Map, dt: f32) -> Option<Vec2> {
        if !self.active {
            return None;
        }
        let mut remaining_time = dt;
        while remaining_time > f32::EPSILON {
            let step_dt = remaining_time.min(PROJECTILE_SUBSTEP_DT);
            remaining_time -= step_dt;

            self.velocity.y += GRAVITY * step_dt;
            self.pos += self.velocity * step_dt;

            if self.pos.x < 0.0
                || self.pos.x > map.total_width()
                || self.pos.y < 0.0
                || self.pos.y > map.total_height()
            {
                self.active = false;
                return None;
            }

            if let Some((row, col)) = map.world_to_grid(self.pos.x, self.pos.y) {
                if map.is_solid(row, col) {
                    self.active = false;
                    return Some(self.pos);
                }
            }
        }

        None
    }

    fn draw(&self) {
        if self.active {
            draw_circle(self.pos.x, self.pos.y, PROJECTILE_RADIUS, RED);
        }
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut map = Map::from_csv(MAP_PATH);
    let total_width = map.total_width();
    let mut players = vec![
        Player::spawn_with_hint(&map, 0, GOLD, total_width * 0.25, "玩家一"),
        Player::spawn_with_hint(&map, 1, SKYBLUE, total_width * 0.75, "玩家二"),
    ];
    let mut active_index: usize = 0;
    let mut turn_timer = TURN_DURATION;
    let mut projectile = Projectile {
        pos: Vec2::ZERO,
        velocity: Vec2::ZERO,
        active: false,
        owner_id: usize::MAX,
    };
    let mut airpack_mode = false;

    loop {
        let dt = get_frame_time();
        if !ensure_active_player_alive(&mut active_index, &players) {
            draw_game_over_scene(&map, &players);
            next_frame().await;
            continue;
        }

        let controls_enabled = players[active_index].is_alive();
        let button_rect = airpack_button_rect();
        if controls_enabled && is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if button_rect.contains(vec2(mx, my)) {
                airpack_mode = !airpack_mode;
            }
        } else if !controls_enabled {
            airpack_mode = false;
        }

        for idx in 0..players.len() {
            let enable = idx == active_index && controls_enabled;
            players[idx].update(&map, dt, enable);
        }

        if controls_enabled && players[active_index].is_charging && is_key_released(KeyCode::Space) {
            let charge = players[active_index].charge_power;
            if charge > 0.0 {
                if airpack_mode {
                    players[active_index].launch_self(charge);
                    airpack_mode = false;
                } else {
                    let origin = vec2(players[active_index].pos.x, players[active_index].pos.y - PLAYER_HEIGHT * 0.5);
                    projectile = Projectile::launch(
                        origin,
                        players[active_index].facing,
                        players[active_index].launch_angle,
                        charge,
                        active_index,
                    );
                }
            }
            players[active_index].cancel_charge();
        }

        if projectile.active {
            if let Some(hit_pos) = projectile.update(&map, dt) {
                map.carve_circle(hit_pos, EXPLOSION_RADIUS);
                apply_explosion_damage(hit_pos, &mut players);
            }
        }

        if !players[active_index].is_alive() {
            advance_turn(&mut active_index, &mut turn_timer, &mut airpack_mode, &mut players);
        } else {
            turn_timer -= dt;
            if turn_timer <= 0.0 {
                advance_turn(&mut active_index, &mut turn_timer, &mut airpack_mode, &mut players);
            }
        }

        clear_background(BLACK);
        map.draw();
        for idx in 0..players.len() {
            let is_active = idx == active_index && players[idx].is_alive();
            players[idx].draw(is_active);
        }
        projectile.draw();
        let active_alive_now = players[active_index].is_alive();
        let charge_player = players.get(active_index).filter(|p| p.is_alive());
        draw_charge_bar(charge_player);
        draw_airpack_button(airpack_mode, active_alive_now);
        draw_turn_timer(turn_timer, players.get(active_index).filter(|p| p.is_alive()).map(|p| p.name));

        next_frame().await;
    }
}

fn draw_charge_bar(player: Option<&Player>) {
    let ratio = player
        .map(|p| (p.charge_power / POWER_MAX).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let x = (screen_width() - CHARGE_BAR_WIDTH) * 0.5;
    let y = CHARGE_BAR_MARGIN;
    draw_rectangle(x, y, CHARGE_BAR_WIDTH, CHARGE_BAR_HEIGHT, DARKGRAY);
    if ratio > 0.0 {
        draw_rectangle(
            x,
            y,
            CHARGE_BAR_WIDTH * ratio,
            CHARGE_BAR_HEIGHT,
            RED,
        );
    }
    draw_rectangle_lines(x, y, CHARGE_BAR_WIDTH, CHARGE_BAR_HEIGHT, 2.0, WHITE);

    if let Some(p) = player {
        let label = format!("{} 蓄力", p.name);
        let dims = measure_text(&label, None, 18, 1.0);
        draw_text(
            &label,
            x + (CHARGE_BAR_WIDTH - dims.width) * 0.5,
            y - 6.0,
            18.0,
            WHITE,
        );
    } else {
        let label = "等待下一位";
        let dims = measure_text(label, None, 18, 1.0);
        draw_text(
            label,
            x + (CHARGE_BAR_WIDTH - dims.width) * 0.5,
            y - 6.0,
            18.0,
            GRAY,
        );
    }
}

fn airpack_button_rect() -> Rect {
    let x = AIRPACK_BTN_MARGIN;
    let y = screen_height() - AIRPACK_BTN_MARGIN - AIRPACK_BTN_HEIGHT;
    Rect::new(x, y, AIRPACK_BTN_WIDTH, AIRPACK_BTN_HEIGHT)
}

fn draw_airpack_button(toggled: bool, enabled: bool) {
    let rect = airpack_button_rect();
    let base_color = if !enabled {
        GRAY
    } else if toggled {
        ORANGE
    } else {
        DARKGRAY
    };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, base_color);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, WHITE);

    let label = "空气背包";
    let text_size = 24;
    let text_width = measure_text(label, None, text_size, 1.0).width;
    let text_x = rect.x + (rect.w - text_width) * 0.5;
    let text_y = rect.y + rect.h * 0.65;
    let text_color = if enabled { WHITE } else { LIGHTGRAY };
    draw_text(label, text_x, text_y, text_size as f32, text_color);
}

fn draw_turn_timer(time_remaining: f32, active_name: Option<&str>) {
    let seconds = time_remaining.max(0.0).ceil();
    let label = if let Some(name) = active_name {
        format!("{} 回合剩余 {:.0}s", name, seconds)
    } else {
        "等待玩家".to_string()
    };
    let dims = measure_text(&label, None, 24, 1.0);
    let x = screen_width() - dims.width - 24.0;
    let y = CHARGE_BAR_MARGIN + CHARGE_BAR_HEIGHT + 12.0;
    draw_text(&label, x, y, 24.0, WHITE);
}

fn apply_explosion_damage(center: Vec2, players: &mut [Player]) {
    for player in players.iter_mut() {
        if !player.is_alive() {
            continue;
        }
        let torso_center = vec2(player.pos.x, player.pos.y - PLAYER_HEIGHT * 0.5);
        let distance = torso_center.distance(center);
        let effective_radius = EXPLOSION_RADIUS + PLAYER_WIDTH * 0.5;
        if distance <= effective_radius {
            let falloff = 1.0 - (distance / effective_radius).clamp(0.0, 1.0);
            let damage = EXPLOSION_DAMAGE * falloff;
            if damage > 0.0 {
                player.apply_damage(damage);
            }
        }
    }
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

fn advance_turn(
    active_index: &mut usize,
    turn_timer: &mut f32,
    airpack_mode: &mut bool,
    players: &mut [Player],
) {
    if let Some(active) = players.get_mut(*active_index) {
        active.cancel_charge();
    }
    *airpack_mode = false;
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

fn draw_game_over_scene(map: &Map, players: &[Player]) {
    clear_background(BLACK);
    map.draw();
    for player in players {
        player.draw(false);
    }
    let message = "游戏结束";
    let dims = measure_text(message, None, 36, 1.0);
    draw_text(
        message,
        (screen_width() - dims.width) * 0.5,
        screen_height() * 0.5,
        36.0,
        WHITE,
    );
}
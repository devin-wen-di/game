use macroquad::prelude::*;
use macroquad::rand::gen_range;

const MAP_PATH: &str = "src/map.csv";
const TILE_SIZE: f32 = 2.0;
const COLS: usize = 768;
const ROWS: usize = 512;
const PLAYER_WIDTH: f32 = TILE_SIZE * 4.0;
const PLAYER_HEIGHT: f32 = TILE_SIZE * 8.0;
const MOVE_SPEED: f32 = 120.0;
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
const CHARGE_BAR_WIDTH: f32 = 220.0;
const CHARGE_BAR_HEIGHT: f32 = 12.0;
const CHARGE_BAR_MARGIN: f32 = 16.0;
const AIRPACK_BTN_WIDTH: f32 = 140.0;
const AIRPACK_BTN_HEIGHT: f32 = 40.0;
const AIRPACK_BTN_MARGIN: f32 = 16.0;

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
}

impl Player {
    fn spawn(map: &Map) -> Self {
        let min_x = TILE_SIZE * 0.5;
        let max_x = map.max_x();
        let mut spawn_x = (min_x + max_x) * 0.5;
        for _ in 0..128 {
            let candidate = gen_range(min_x, max_x);
            let left = candidate - PLAYER_WIDTH * 0.5;
            let right = candidate + PLAYER_WIDTH * 0.5;
            if map.find_floor_below(left, right, -PLAYER_HEIGHT).is_some() {
                spawn_x = candidate;
                break;
            }
        }

        Self {
            pos: vec2(spawn_x, PLAYER_HEIGHT),
            velocity: vec2(0.0, 0.0),
            on_ground: false,
            facing: 1.0,
            launch_angle: MIN_LAUNCH_ANGLE,
            is_charging: false,
            charge_power: 0.0,
            is_airpack_flight: false,
        }
    }

    fn reset(&mut self, map: &Map) {
        *self = Player::spawn(map);
    }

    fn update(&mut self, map: &Map, dt: f32) {
        if self.is_airpack_flight {
            // preserve launch velocity while airborne due to airpack skill
        } else if !self.is_charging {
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

        if is_key_down(KeyCode::Up) {
            self.launch_angle = (self.launch_angle + 1.2 * dt).min(MAX_LAUNCH_ANGLE);
        }
        if is_key_down(KeyCode::Down) {
            self.launch_angle = (self.launch_angle - 1.2 * dt).max(MIN_LAUNCH_ANGLE);
        }

        if is_key_pressed(KeyCode::Space) {
            self.is_charging = true;
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

    fn draw(&self) {
        let x = self.pos.x - PLAYER_WIDTH * 0.5;
        let y = self.pos.y - PLAYER_HEIGHT;
        draw_rectangle(x, y, PLAYER_WIDTH, PLAYER_HEIGHT, YELLOW);

        let aim_origin = vec2(self.pos.x, self.pos.y - PLAYER_HEIGHT * 0.5);
        let direction = vec2(self.facing * self.launch_angle.cos(), -self.launch_angle.sin());
        let aim_end = aim_origin + direction.normalize() * 40.0;
        draw_line(aim_origin.x, aim_origin.y, aim_end.x, aim_end.y, 2.0, WHITE);
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
}

struct Projectile {
    pos: Vec2,
    velocity: Vec2,
    active: bool,
}

impl Projectile {
    fn launch(origin: Vec2, facing: f32, angle: f32, power: f32) -> Self {
        let dir = vec2(facing * angle.cos(), -angle.sin()).normalize();
        Self {
            pos: origin,
            velocity: dir * power,
            active: true,
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
    let mut player = Player::spawn(&map);
    let mut projectile = Projectile {
        pos: Vec2::ZERO,
        velocity: Vec2::ZERO,
        active: false,
    };
    let mut airpack_mode = false;

    loop {
        let dt = get_frame_time();
        let button_rect = airpack_button_rect();
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mx, my) = mouse_position();
            if button_rect.contains(vec2(mx, my)) {
                airpack_mode = !airpack_mode;
            }
        }

        player.update(&map, dt);

        if player.is_charging && is_key_released(KeyCode::Space) {
            if player.charge_power > 0.0 {
                if airpack_mode {
                    player.launch_self(player.charge_power);
                    airpack_mode = false;
                } else {
                    let origin = vec2(player.pos.x, player.pos.y - PLAYER_HEIGHT * 0.5);
                    projectile = Projectile::launch(origin, player.facing, player.launch_angle, player.charge_power);
                }
                player.is_charging = false;
                player.charge_power = 0.0;
            } else {
                player.is_charging = false;
            }
        }

        if projectile.active {
            if let Some(hit_pos) = projectile.update(&map, dt) {
                map.carve_circle(hit_pos, EXPLOSION_RADIUS);
            }
        }

        clear_background(BLACK);
        map.draw();
        player.draw();
        projectile.draw();
        draw_charge_bar(&player);
        draw_airpack_button(airpack_mode);

        next_frame().await;
    }
}

fn draw_charge_bar(player: &Player) {
    let ratio = (player.charge_power / POWER_MAX).clamp(0.0, 1.0);
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
}

fn airpack_button_rect() -> Rect {
    let x = AIRPACK_BTN_MARGIN;
    let y = screen_height() - AIRPACK_BTN_MARGIN - AIRPACK_BTN_HEIGHT;
    Rect::new(x, y, AIRPACK_BTN_WIDTH, AIRPACK_BTN_HEIGHT)
}

fn draw_airpack_button(active: bool) {
    let rect = airpack_button_rect();
    let base_color = if active { ORANGE } else { DARKGRAY };
    draw_rectangle(rect.x, rect.y, rect.w, rect.h, base_color);
    draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, WHITE);

    let label = "jet pack";
    let text_size = 26;
    let text_width = measure_text(label, None, text_size, 1.0).width;
    let text_x = rect.x + (rect.w - text_width) * 0.5;
    let text_y = rect.y + rect.h * 0.6;
    draw_text(label, text_x, text_y, text_size as f32, WHITE);
}
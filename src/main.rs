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
const SKILL_BUTTON_GAP: f32 = 12.0;
const SCATTER_OFFSET_RAD: f32 = 5.0_f32.to_radians();
const TRIPLE_DELAY: f32 = 0.5;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum SkillKind {
    Jetpack,
    Heavy,
    Triple,
    Scatter,
}

#[derive(Clone, Default)]
struct SkillLoadout {
    entries: Vec<SkillKind>,
}

impl SkillLoadout {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn jetpack_enabled(&self) -> bool {
        self.entries.contains(&SkillKind::Jetpack)
    }

    fn heavy_count(&self) -> usize {
        self.entries.iter().filter(|&&s| s == SkillKind::Heavy).count()
    }

    fn triple_count(&self) -> usize {
        self.entries.iter().filter(|&&s| s == SkillKind::Triple).count()
    }

    fn scatter_selected(&self) -> bool {
        self.entries.contains(&SkillKind::Scatter)
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn total_count(&self) -> usize {
        self.entries.len()
    }

    fn toggle(&mut self, skill: SkillKind) {
        match skill {
            SkillKind::Jetpack => {
                if self.jetpack_enabled() {
                    self.entries.clear();
                } else {
                    self.entries.clear();
                    self.entries.push(SkillKind::Jetpack);
                }
            }
            SkillKind::Scatter => {
                if self.jetpack_enabled() {
                    return;
                }
                if self.entries.contains(&SkillKind::Scatter) {
                    self.entries.retain(|&s| s != SkillKind::Scatter);
                } else {
                    let mut candidate = self.entries.clone();
                    candidate.push(SkillKind::Scatter);
                    if is_valid_skill_combo(&candidate) {
                        self.entries = candidate;
                    }
                }
            }
            SkillKind::Heavy => {
                if self.jetpack_enabled() {
                    return;
                }
                let current = self.heavy_count();
                let mut base = self.entries.clone();
                base.retain(|&s| s != SkillKind::Heavy);
                match current {
                    0 => {
                        let mut candidate = self.entries.clone();
                        candidate.push(SkillKind::Heavy);
                        if is_valid_skill_combo(&candidate) {
                            self.entries = candidate;
                        }
                    }
                    1 => {
                        let mut candidate = self.entries.clone();
                        candidate.push(SkillKind::Heavy);
                        if is_valid_skill_combo(&candidate) {
                            self.entries = candidate;
                        } else {
                            self.entries = base;
                        }
                    }
                    _ => {
                        self.entries = base;
                    }
                }
            }
            SkillKind::Triple => {
                if self.jetpack_enabled() {
                    return;
                }
                let current = self.triple_count();
                let mut base = self.entries.clone();
                base.retain(|&s| s != SkillKind::Triple);
                match current {
                    0 => {
                        let mut candidate = self.entries.clone();
                        candidate.push(SkillKind::Triple);
                        if is_valid_skill_combo(&candidate) {
                            self.entries = candidate;
                        }
                    }
                    1 => {
                        let mut candidate = self.entries.clone();
                        candidate.push(SkillKind::Triple);
                        if is_valid_skill_combo(&candidate) {
                            self.entries = candidate;
                        } else {
                            self.entries = base;
                        }
                    }
                    _ => {
                        self.entries = base;
                    }
                }
            }
        }
        self.entries.sort_by_key(|s| match s {
            SkillKind::Jetpack => 0,
            SkillKind::Heavy => 1,
            SkillKind::Triple => 2,
            SkillKind::Scatter => 3,
        });
    }

    fn labels(&self) -> Vec<&'static str> {
        if self.entries.is_empty() {
            return Vec::new();
        }
        let mut result = Vec::new();
        for skill in &self.entries {
            match skill {
                SkillKind::Jetpack => result.push("jet"),
                SkillKind::Heavy => result.push("heavy"),
                SkillKind::Triple => result.push("consecutive"),
                SkillKind::Scatter => result.push("scatter"),
            }
        }
        result
    }
}

fn is_valid_skill_combo(skills: &[SkillKind]) -> bool {
    if skills.is_empty() {
        return true;
    }
    if skills.len() > 2 {
        return false;
    }
    if skills.contains(&SkillKind::Jetpack) {
        return skills.len() == 1;
    }

    let heavy = skills.iter().filter(|&&s| s == SkillKind::Heavy).count();
    let triple = skills.iter().filter(|&&s| s == SkillKind::Triple).count();
    let scatter = skills.iter().any(|&s| s == SkillKind::Scatter);

    match (heavy, triple, scatter) {
        (0, 0, false) => true,
        (1, 0, false) => true,
        (2, 0, false) => true,
        (0, 1, false) => true,
        (0, 2, false) => true,
        (0, 0, true) => true,
        (1, 1, false) => true,
        (1, 0, true) => true,
        (0, 1, true) => true,
        _ => false,
    }
}

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

fn build_shot_pattern(skills: &SkillLoadout) -> (Vec<(f32, Vec<f32>)>, f32) {
    let heavy_count = skills.heavy_count();
    let triple_count = skills.triple_count();
    let scatter_selected = skills.scatter_selected();

    let damage_multiplier = match heavy_count {
        0 => 1.0,
        1 => 1.5,
        _ => 2.0,
    };

    let mut pattern: Vec<(f32, Vec<f32>)> = Vec::new();

    match triple_count {
        0 => {
            if scatter_selected {
                pattern.push((0.0, vec![-SCATTER_OFFSET_RAD, 0.0, SCATTER_OFFSET_RAD]));
            }
        }
        1 => {
            for idx in 0..3 {
                let offsets = if scatter_selected {
                    vec![-SCATTER_OFFSET_RAD, 0.0, SCATTER_OFFSET_RAD]
                } else {
                    vec![0.0]
                };
                pattern.push((TRIPLE_DELAY * idx as f32, offsets));
            }
        }
        _ => {
            for idx in 0..5 {
                let offsets = if scatter_selected {
                    vec![-SCATTER_OFFSET_RAD, 0.0, SCATTER_OFFSET_RAD]
                } else {
                    vec![0.0]
                };
                pattern.push((TRIPLE_DELAY * idx as f32, offsets));
            }
        }
    }

    (pattern, damage_multiplier)
}

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
    skills: SkillLoadout,
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
            skills: SkillLoadout::new(),
        }
    }

    fn reset(&mut self, map: &Map) {
        let snapshot = Player::spawn_with_hint(map, self.id, self.color, self.spawn_hint, self.name);
        let preserved_health = self.health;
        let skills = self.skills.clone();
        *self = snapshot;
        self.health = preserved_health;
        self.skills = skills;
    }

    fn update(&mut self, map: &Map, dt: f32, controls_enabled: bool) {
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
        let (body_width, body_height, body_top) = if self.is_alive() {
            (PLAYER_WIDTH, PLAYER_HEIGHT, self.pos.y - PLAYER_HEIGHT)
        } else {
            (PLAYER_HEIGHT, PLAYER_WIDTH, self.pos.y - PLAYER_WIDTH)
        };
        let x = self.pos.x - body_width * 0.5;
        let body_color = if self.is_alive() {
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

        draw_rectangle(x, body_top, body_width, body_height, body_color);
        if active && self.is_alive() {
            draw_rectangle_lines(x - 2.0, body_top - 2.0, body_width + 4.0, body_height + 4.0, 2.0, WHITE);
        }

        if self.is_alive() {
            let aim_origin = vec2(self.pos.x, self.pos.y - PLAYER_HEIGHT * 0.5);
            let direction = vec2(self.facing * self.launch_angle.cos(), -self.launch_angle.sin());
            let aim_end = aim_origin + direction.normalize() * 40.0;
            draw_line(aim_origin.x, aim_origin.y, aim_end.x, aim_end.y, 2.0, WHITE);
        }

        let bar_x = self.pos.x - HEALTH_BAR_WIDTH * 0.5;
        let bar_y = body_top - HEALTH_BAR_HEIGHT - 6.0;
        draw_rectangle(bar_x, bar_y, HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT, DARKGRAY);
        let health_ratio = (self.health / MAX_HEALTH).clamp(0.0, 1.0);
        if health_ratio > 0.0 {
            draw_rectangle(bar_x, bar_y, HEALTH_BAR_WIDTH * health_ratio, HEALTH_BAR_HEIGHT, GREEN);
        }
        draw_rectangle_lines(bar_x, bar_y, HEALTH_BAR_WIDTH, HEALTH_BAR_HEIGHT, 1.0, WHITE);

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
        let previous = self.health;
        self.health = (self.health - amount).max(0.0);
        if previous > 0.0 && self.health <= 0.0 {
            self.cancel_charge();
            self.velocity = Vec2::ZERO;
            self.is_airpack_flight = false;
        }
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
    damage: f32,
}

impl Projectile {
    fn launch(origin: Vec2, facing: f32, angle: f32, power: f32, owner_id: usize, damage: f32) -> Self {
        let dir = vec2(facing * angle.cos(), -angle.sin()).normalize();
        Self {
            pos: origin,
            velocity: dir * power,
            active: true,
            owner_id,
            damage,
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
        Player::spawn_with_hint(&map, 0, GOLD, total_width * 0.25, "Player 1"),
        Player::spawn_with_hint(&map, 1, SKYBLUE, total_width * 0.75, "Player 2"),
    ];
    let mut active_index: usize = 0;
    let mut turn_timer = TURN_DURATION;
    let mut projectiles: Vec<Projectile> = Vec::new();
    let mut pending_shots: Vec<ShotTask> = Vec::new();
    let mut pending_turn_after_action = false;

    loop {
        let dt = get_frame_time();
        if !ensure_active_player_alive(&mut active_index, &players) {
            draw_game_over_scene(&map, &players);
            next_frame().await;
            continue;
        }

        let controls_enabled = players[active_index].is_alive();

        if controls_enabled && is_mouse_button_pressed(MouseButton::Left) {
            let mouse = vec2(mouse_position().0, mouse_position().1);
            handle_skill_button_click(mouse, &mut players[active_index]);
        }

        for idx in 0..players.len() {
            let enable = idx == active_index && controls_enabled;
            players[idx].update(&map, dt, enable);
        }

        if controls_enabled && players[active_index].is_charging && is_key_released(KeyCode::Space) {
            let charge = players[active_index].charge_power;
            if charge > 0.0 {
                if players[active_index].skills.jetpack_enabled() {
                    players[active_index].launch_self(charge);
                    pending_turn_after_action = true;
                } else {
                    let base_angle = players[active_index].launch_angle;
                    let origin = vec2(
                        players[active_index].pos.x,
                        players[active_index].pos.y - PLAYER_HEIGHT * 0.5,
                    );
                    let facing = players[active_index].facing;
                    let (pattern, damage_multiplier) = build_shot_pattern(&players[active_index].skills);
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

        let mut ready_tasks: Vec<ShotTask> = Vec::new();
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
            for offset in &task.angle_offsets {
                let mut angle = task.base_angle + *offset;
                angle = angle.clamp(MIN_LAUNCH_ANGLE, MAX_LAUNCH_ANGLE);
                projectiles.push(Projectile::launch(
                    task.origin,
                    task.facing,
                    angle,
                    task.power,
                    task.owner_idx,
                    task.damage,
                ));
            }
        }

        let mut impacts: Vec<(Vec2, f32)> = Vec::new();
        for proj in projectiles.iter_mut() {
            if !proj.active {
                continue;
            }
            if let Some(hit_pos) = proj.update(&map, dt) {
                impacts.push((hit_pos, proj.damage));
                proj.active = false;
            }
        }

        for (hit_pos, damage) in impacts {
            map.carve_circle(hit_pos, EXPLOSION_RADIUS);
            apply_explosion_damage(hit_pos, damage, &mut players);
        }

        projectiles.retain(|p| p.active);

        if players[active_index].is_alive() {
            if !pending_turn_after_action {
                turn_timer -= dt;
            }
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
        for proj in &projectiles {
            proj.draw();
        }
        let charge_player = players.get(active_index).filter(|p| p.is_alive());
        draw_charge_bar(charge_player);
        draw_skill_buttons(players.get(active_index));
        draw_turn_timer(
            turn_timer,
            players
                .get(active_index)
                .filter(|p| p.is_alive())
                .map(|p| p.name),
        );

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
        let label = format!("{} Ready", p.name);
        let dims = measure_text(&label, None, 18, 1.0);
        draw_text(
            &label,
            x + (CHARGE_BAR_WIDTH - dims.width) * 0.5,
            y - 6.0,
            18.0,
            WHITE,
        );
    } else {
        let label = "wait for next turn";
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

const SKILL_BUTTONS: [(SkillKind, &str); 4] = [
    (SkillKind::Jetpack, "jet"),
    (SkillKind::Heavy, "heavy"),
    (SkillKind::Triple, "consecutive"),
    (SkillKind::Scatter, "scatter"),
];

fn skill_button_rect(index: usize) -> Rect {
    let start_x = AIRPACK_BTN_MARGIN;
    let y = screen_height() - AIRPACK_BTN_MARGIN - AIRPACK_BTN_HEIGHT;
    let x = start_x + index as f32 * (AIRPACK_BTN_WIDTH + SKILL_BUTTON_GAP);
    Rect::new(x, y, AIRPACK_BTN_WIDTH, AIRPACK_BTN_HEIGHT)
}

fn draw_skill_buttons(player: Option<&Player>) {
    let (jetpack_enabled, heavy_count, triple_count, scatter_selected) = if let Some(p) = player {
        (
            p.skills.jetpack_enabled(),
            p.skills.heavy_count(),
            p.skills.triple_count(),
            p.skills.scatter_selected(),
        )
    } else {
        (false, 0, 0, false)
    };

    for (index, (skill, base_label)) in SKILL_BUTTONS.iter().enumerate() {
        let rect = skill_button_rect(index);
        let enabled = player.map_or(false, |p| p.is_alive());
        let (selected, label) = match skill {
            SkillKind::Jetpack => (jetpack_enabled, base_label.to_string()),
            SkillKind::Heavy => (
                heavy_count > 0,
                if heavy_count > 1 {
                    format!("{} x{}", base_label, heavy_count)
                } else {
                    base_label.to_string()
                },
            ),
            SkillKind::Triple => (
                triple_count > 0,
                if triple_count > 1 {
                    format!("{} x{}", base_label, triple_count)
                } else {
                    base_label.to_string()
                },
            ),
            SkillKind::Scatter => (scatter_selected, base_label.to_string()),
        };

        let base_color = if !enabled {
            GRAY
        } else if selected {
            ORANGE
        } else {
            DARKGRAY
        };

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, base_color);
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, WHITE);

        let text_size = 22;
        let dims = measure_text(&label, None, text_size, 1.0);
        let text_x = rect.x + (rect.w - dims.width) * 0.5;
        let text_y = rect.y + rect.h * 0.65;
        let text_color = if enabled { WHITE } else { LIGHTGRAY };
        draw_text(&label, text_x, text_y, text_size as f32, text_color);
    }
}

fn handle_skill_button_click(mouse: Vec2, player: &mut Player) -> bool {
    for (index, (skill, _)) in SKILL_BUTTONS.iter().enumerate() {
        let rect = skill_button_rect(index);
        if rect.contains(mouse) {
            player.skills.toggle(*skill);
            return true;
        }
    }
    false
}

fn draw_turn_timer(time_remaining: f32, active_name: Option<&str>) {
    let seconds = time_remaining.max(0.0).ceil();
    let label = if let Some(name) = active_name {
        format!("{} round remain {:.0}s", name, seconds)
    } else {
        "wait for player".to_string()
    };
    let dims = measure_text(&label, None, 24, 1.0);
    let x = screen_width() - dims.width - 24.0;
    let y = CHARGE_BAR_MARGIN + CHARGE_BAR_HEIGHT + 12.0;
    draw_text(&label, x, y, 24.0, WHITE);
}

fn apply_explosion_damage(center: Vec2, base_damage: f32, players: &mut [Player]) {
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

fn draw_game_over_scene(map: &Map, players: &[Player]) {
    clear_background(BLACK);
    map.draw();
    for player in players {
        player.draw(false);
    }
    let message = "Game Over";
    let dims = measure_text(message, None, 36, 1.0);
    draw_text(
        message,
        (screen_width() - dims.width) * 0.5,
        screen_height() * 0.5,
        36.0,
        WHITE,
    );
}
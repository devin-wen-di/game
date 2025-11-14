use macroquad::prelude::*;

use crate::config::{GRAVITY, PROJECTILE_RADIUS, PROJECTILE_SUBSTEP_DT};
use crate::map::Map;

pub struct Projectile {
    pos: Vec2,
    velocity: Vec2,
    active: bool,
    damage: f32,
}

impl Projectile {
    pub fn launch(origin: Vec2, facing: f32, angle: f32, power: f32, damage: f32) -> Self {
        let dir = vec2(facing * angle.cos(), -angle.sin()).normalize();
        Self {
            pos: origin,
            velocity: dir * power,
            active: true,
            damage,
        }
    }

    pub fn update(&mut self, map: &Map, dt: f32) -> Option<Vec2> {
        if !self.active {
            return None;
        }
        let mut remaining_time = dt;
        while remaining_time > f32::EPSILON {
            let step_dt = remaining_time.min(PROJECTILE_SUBSTEP_DT);
            remaining_time -= step_dt;

            self.velocity.y += GRAVITY * step_dt;
            self.pos += self.velocity * step_dt;

            if self.pos.x < 0.0 || self.pos.x > map.total_width() || self.pos.y > map.total_height()
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

    pub fn draw(&self) {
        if self.active {
            draw_circle(self.pos.x, self.pos.y, PROJECTILE_RADIUS, RED);
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn deactivate(&mut self) {
        self.active = false;
    }

    pub fn damage(&self) -> f32 {
        self.damage
    }
}

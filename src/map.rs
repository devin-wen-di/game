use macroquad::prelude::*;

use crate::config::TILE_SIZE;

#[derive(Clone)]
pub struct Map {
    tiles: Vec<Vec<u8>>,
    rows: usize,
    cols: usize,
}

impl Map {
    pub fn from_csv(path: &str) -> Self {
        let content =
            std::fs::read_to_string(path).unwrap_or_else(|err| panic!("读取 {path} 失败: {err}"));
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

    pub fn total_width(&self) -> f32 {
        self.cols as f32 * TILE_SIZE
    }

    pub fn total_height(&self) -> f32 {
        self.rows as f32 * TILE_SIZE
    }

    pub fn is_solid(&self, row: usize, col: usize) -> bool {
        self.tiles
            .get(row)
            .and_then(|r| r.get(col))
            .map(|v| *v != 0)
            .unwrap_or(false)
    }

    pub fn max_x(&self) -> f32 {
        (self.cols as f32 - 0.5) * TILE_SIZE
    }

    pub fn draw(&self) {
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

    pub fn find_floor_below(&self, left: f32, right: f32, from_y: f32) -> Option<f32> {
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

    pub fn world_to_grid(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let col = (x / TILE_SIZE).floor() as isize;
        let screen_row = (y / TILE_SIZE).floor() as isize;
        if col < 0
            || screen_row < 0
            || col as usize >= self.cols
            || screen_row as usize >= self.rows
        {
            None
        } else {
            let row = self.rows as isize - 1 - screen_row;
            Some((row as usize, col as usize))
        }
    }

    pub fn carve_circle(&mut self, center: Vec2, radius: f32) {
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

mod config;
mod game;
mod map;
mod player;
mod projectile;
mod skills;
mod ui;

use game::window_conf;

#[macroquad::main(window_conf)]
async fn main() {
    game::run_game().await;
}

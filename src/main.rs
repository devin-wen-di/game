use my_game::{LaunchOptions, run_game, window_conf};

#[macroquad::main(window_conf)]
async fn main() {
    let options = LaunchOptions::from_env();
    run_game(options).await;
}

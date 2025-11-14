use macroquad::prelude::*;

use crate::config::{
    AIRPACK_BTN_HEIGHT, AIRPACK_BTN_MARGIN, AIRPACK_BTN_WIDTH, CHARGE_BAR_HEIGHT,
    CHARGE_BAR_MARGIN, CHARGE_BAR_WIDTH, POWER_MAX, SKILL_BUTTON_GAP,
};
use crate::map::Map;
use crate::player::Player;
use crate::skills::SkillKind;

pub fn draw_charge_bar(player: Option<&Player>) {
    let ratio = player
        .map(|p| (p.charge_power() / POWER_MAX).clamp(0.0, 1.0))
        .unwrap_or(0.0);
    let x = (screen_width() - CHARGE_BAR_WIDTH) * 0.5;
    let y = CHARGE_BAR_MARGIN;
    draw_rectangle(x, y, CHARGE_BAR_WIDTH, CHARGE_BAR_HEIGHT, DARKGRAY);
    if ratio > 0.0 {
        draw_rectangle(x, y, CHARGE_BAR_WIDTH * ratio, CHARGE_BAR_HEIGHT, RED);
    }
    draw_rectangle_lines(x, y, CHARGE_BAR_WIDTH, CHARGE_BAR_HEIGHT, 2.0, WHITE);

    if let Some(p) = player {
        let label = format!("{} Ready", p.name());
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

pub fn draw_skill_buttons(player: Option<&Player>) {
    let (jetpack_enabled, heavy_count, triple_count, scatter_selected) = if let Some(p) = player {
        (
            p.skills().jetpack_enabled(),
            p.skills().heavy_count(),
            p.skills().triple_count(),
            p.skills().scatter_selected(),
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

pub fn handle_skill_button_click(mouse: Vec2, player: &mut Player) -> bool {
    for (index, (skill, _)) in SKILL_BUTTONS.iter().enumerate() {
        let rect = skill_button_rect(index);
        if rect.contains(mouse) {
            player.skills_mut().toggle(*skill);
            return true;
        }
    }
    false
}

pub fn draw_turn_timer(time_remaining: f32, active_name: Option<&str>) {
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

pub fn draw_game_over_scene(map: &Map, players: &[Player], winner: Option<&str>) {
    map.draw();
    for player in players {
        player.draw(false);
    }

    let message = winner
        .map(|name| format!("{name} wins!"))
        .unwrap_or_else(|| "Game Over".to_string());
    let dims = measure_text(&message, None, 36, 1.0);
    draw_text(
        &message,
        (screen_width() - dims.width) * 0.5,
        screen_height() * 0.5,
        36.0,
        WHITE,
    );

    let hint = "Press Enter or click to return";
    let hint_dims = measure_text(hint, None, 20, 1.0);
    draw_text(
        hint,
        (screen_width() - hint_dims.width) * 0.5,
        screen_height() * 0.5 + 40.0,
        20.0,
        LIGHTGRAY,
    );
}

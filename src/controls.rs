use serde::{Deserialize, Serialize};

use crate::skills::SkillKind;

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct ControlState {
    pub move_axis: i8,
    pub aim_up: bool,
    pub aim_down: bool,
    pub start_charge: bool,
    pub release_charge: bool,
    pub skill_command: Option<SkillCommand>,
}

impl ControlState {
    pub fn idle() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum SkillCommand {
    Toggle(SkillKind),
}

impl SkillCommand {
    pub fn kind(self) -> SkillKind {
        match self {
            SkillCommand::Toggle(kind) => kind,
        }
    }
}

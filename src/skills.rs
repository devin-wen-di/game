use crate::config::{SCATTER_OFFSET_RAD, TRIPLE_DELAY};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
pub enum SkillKind {
    Jetpack,
    Heavy,
    Triple,
    Scatter,
}

#[derive(Clone, Default)]
pub struct SkillLoadout {
    entries: Vec<SkillKind>,
}

impl SkillLoadout {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub fn jetpack_enabled(&self) -> bool {
        self.entries.contains(&SkillKind::Jetpack)
    }

    pub fn heavy_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|&&s| s == SkillKind::Heavy)
            .count()
    }

    pub fn triple_count(&self) -> usize {
        self.entries
            .iter()
            .filter(|&&s| s == SkillKind::Triple)
            .count()
    }

    pub fn scatter_selected(&self) -> bool {
        self.entries.contains(&SkillKind::Scatter)
    }

    pub fn toggle(&mut self, skill: SkillKind) {
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

    pub fn labels(&self) -> Vec<&'static str> {
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

pub fn is_valid_skill_combo(skills: &[SkillKind]) -> bool {
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

pub fn build_shot_pattern(loadout: &SkillLoadout) -> (Vec<(f32, Vec<f32>)>, f32) {
    let heavy_count = loadout.heavy_count();
    let triple_count = loadout.triple_count();
    let scatter_selected = loadout.scatter_selected();

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

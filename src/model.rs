use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolMode {
    Select,
    Arrow,
    Rectangle,
    Ellipse,
    Text,
    StepBadge,
    Blur,
    Pixelate,
    Smudge,
    Crop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObfuscateKind {
    Blur,
    Pixelate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationItem {
    Arrow {
        start: [f32; 2],
        end: [f32; 2],
        color: [u8; 4],
        width: f32,
    },
    Rectangle {
        min: [f32; 2],
        max: [f32; 2],
        color: [u8; 4],
        width: f32,
        fill: Option<[u8; 4]>,
    },
    Ellipse {
        min: [f32; 2],
        max: [f32; 2],
        color: [u8; 4],
        width: f32,
        fill: Option<[u8; 4]>,
    },
    Text {
        pos: [f32; 2],
        text: String,
        color: [u8; 4],
        font_size: f32,
    },
    StepBadge {
        pos: [f32; 2],
        number: u32,
        color: [u8; 4],
        radius: f32,
    },
    Obfuscate {
        min: [f32; 2],
        max: [f32; 2],
        kind: ObfuscateKind,
        strength: u32,
    },
}

impl AnnotationItem {
    #[allow(dead_code)]
    pub fn bounding_box(&self) -> ([f32; 2], [f32; 2]) {
        match self {
            AnnotationItem::Arrow { start, end, .. } => {
                let min_x = start[0].min(end[0]);
                let min_y = start[1].min(end[1]);
                let max_x = start[0].max(end[0]);
                let max_y = start[1].max(end[1]);
                ([min_x, min_y], [max_x, max_y])
            }
            AnnotationItem::Rectangle { min, max, .. }
            | AnnotationItem::Ellipse { min, max, .. }
            | AnnotationItem::Obfuscate { min, max, .. } => {
                let min_x = min[0].min(max[0]);
                let min_y = min[1].min(max[1]);
                let max_x = min[0].max(max[0]);
                let max_y = min[1].max(max[1]);
                ([min_x, min_y], [max_x, max_y])
            }
            AnnotationItem::Text { pos, font_size, text, .. } => {
                let width = (text.len() as f32) * font_size * 0.6;
                let height = *font_size;
                ([pos[0], pos[1]], [pos[0] + width, pos[1] + height])
            }
            AnnotationItem::StepBadge { pos, radius, .. } => (
                [pos[0] - radius, pos[1] - radius],
                [pos[0] + radius, pos[1] + radius],
            ),
        }
    }

    #[allow(dead_code)]
    pub fn translate(&mut self, delta: [f32; 2]) {
        match self {
            AnnotationItem::Arrow { start, end, .. } => {
                start[0] += delta[0];
                start[1] += delta[1];
                end[0] += delta[0];
                end[1] += delta[1];
            }
            AnnotationItem::Rectangle { min, max, .. }
            | AnnotationItem::Ellipse { min, max, .. }
            | AnnotationItem::Obfuscate { min, max, .. } => {
                min[0] += delta[0];
                min[1] += delta[1];
                max[0] += delta[0];
                max[1] += delta[1];
            }
            AnnotationItem::Text { pos, .. } | AnnotationItem::StepBadge { pos, .. } => {
                pos[0] += delta[0];
                pos[1] += delta[1];
            }
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Scene {
    pub items: Vec<AnnotationItem>,
    pub undo_stack: Vec<Vec<AnnotationItem>>,
    pub redo_stack: Vec<Vec<AnnotationItem>>,
    pub next_badge_number: u32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            next_badge_number: 1,
        }
    }

    pub fn push_item(&mut self, item: AnnotationItem) {
        self.undo_stack.push(self.items.clone());
        self.redo_stack.clear();
        if let AnnotationItem::StepBadge { number, .. } = &item {
            self.next_badge_number = self.next_badge_number.max(number + 1);
        }
        self.items.push(item);
    }

    pub fn undo(&mut self) -> bool {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.items.clone());
            self.items = prev;
            self.recalc_badge_number();
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.items.clone());
            self.items = next;
            self.recalc_badge_number();
            true
        } else {
            false
        }
    }

    pub fn clear(&mut self) {
        if !self.items.is_empty() {
            self.undo_stack.push(self.items.clone());
            self.redo_stack.clear();
            self.items.clear();
            self.next_badge_number = 1;
        }
    }

    fn recalc_badge_number(&mut self) {
        let max_n = self
            .items
            .iter()
            .filter_map(|it| match it {
                AnnotationItem::StepBadge { number, .. } => Some(*number),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        self.next_badge_number = max_n + 1;
    }
}

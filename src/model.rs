use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolMode {
    Select,
    Freehand,
    Arrow,
    Rectangle,
    Ellipse,
    Highlight,
    Text,
    StepBadge,
    Blur,
    Pixelate,
    Smudge,
    ColorErase,
    Crop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObfuscateKind {
    Blur,
    Pixelate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontFamily {
    Sans,
    Monospace,
    Serif,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextBackground {
    Halo,
    FilledTag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotationSpan {
    pub start_ms: u64,
    pub end_ms: u64,
}

impl AnnotationSpan {
    pub fn new(start_ms: u64, end_ms: u64) -> Self {
        Self { start_ms, end_ms }
    }

    pub fn is_active_at(&self, time_ms: u64) -> bool {
        time_ms >= self.start_ms && time_ms <= self.end_ms
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineItem {
    pub item: AnnotationItem,
    pub span: Option<AnnotationSpan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationItem {
    Freehand {
        points: Vec<[f32; 2]>,
        color: [u8; 4],
        width: f32,
    },
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
    Highlight {
        min: [f32; 2],
        max: [f32; 2],
        color: [u8; 4],
    },
    Text {
        pos: [f32; 2],
        text: String,
        color: [u8; 4],
        font_size: f32,
        family: FontFamily,
        bg_style: TextBackground,
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
    pub fn layer_name(&self) -> String {
        match self {
            AnnotationItem::Freehand { .. } => "✏ Desenho Livre".into(),
            AnnotationItem::Arrow { .. } => "↗ Seta".into(),
            AnnotationItem::Rectangle { .. } => "▢ Retângulo".into(),
            AnnotationItem::Ellipse { .. } => "◯ Elipse".into(),
            AnnotationItem::Highlight { .. } => "🖊 Grifar".into(),
            AnnotationItem::Text { text, .. } => {
                let preview = if text.len() > 14 {
                    format!("{}...", &text[..14])
                } else {
                    text.clone()
                };
                format!("T \"{}\"", preview)
            }
            AnnotationItem::StepBadge { number, .. } => format!("① Passo {}", number),
            AnnotationItem::Obfuscate { kind, .. } => match kind {
                ObfuscateKind::Blur => "░ Blur".into(),
                ObfuscateKind::Pixelate => "▦ Pixelate".into(),
            },
        }
    }

    pub fn color(&self) -> [u8; 4] {
        match self {
            AnnotationItem::Freehand { color, .. }
            | AnnotationItem::Arrow { color, .. }
            | AnnotationItem::Rectangle { color, .. }
            | AnnotationItem::Ellipse { color, .. }
            | AnnotationItem::Highlight { color, .. }
            | AnnotationItem::Text { color, .. }
            | AnnotationItem::StepBadge { color, .. } => *color,
            AnnotationItem::Obfuscate { .. } => [128, 128, 128, 255],
        }
    }

    pub fn set_color(&mut self, new_color: [u8; 4]) {
        match self {
            AnnotationItem::Freehand { color, .. }
            | AnnotationItem::Arrow { color, .. }
            | AnnotationItem::Rectangle { color, .. }
            | AnnotationItem::Ellipse { color, .. }
            | AnnotationItem::Highlight { color, .. }
            | AnnotationItem::Text { color, .. }
            | AnnotationItem::StepBadge { color, .. } => *color = new_color,
            AnnotationItem::Obfuscate { .. } => {}
        }
    }

    pub fn set_width(&mut self, new_width: f32) {
        match self {
            AnnotationItem::Freehand { width, .. }
            | AnnotationItem::Arrow { width, .. }
            | AnnotationItem::Rectangle { width, .. }
            | AnnotationItem::Ellipse { width, .. } => *width = new_width,
            AnnotationItem::Highlight { .. } => {}
            AnnotationItem::Text { font_size, .. } => *font_size = (new_width * 6.0).max(18.0),
            AnnotationItem::StepBadge { radius, .. } => *radius = (new_width * 3.5).max(12.0),
            AnnotationItem::Obfuscate { strength, .. } => *strength = (new_width * 3.0) as u32,
        }
    }

    pub fn bounding_box(&self) -> ([f32; 2], [f32; 2]) {
        match self {
            AnnotationItem::Freehand { points, .. } => {
                if points.is_empty() {
                    ([0.0, 0.0], [0.0, 0.0])
                } else {
                    let mut min_x = f32::MAX;
                    let mut min_y = f32::MAX;
                    let mut max_x = f32::MIN;
                    let mut max_y = f32::MIN;
                    for p in points {
                        min_x = min_x.min(p[0]);
                        min_y = min_y.min(p[1]);
                        max_x = max_x.max(p[0]);
                        max_y = max_y.max(p[1]);
                    }
                    ([min_x, min_y], [max_x, max_y])
                }
            }
            AnnotationItem::Arrow { start, end, .. } => {
                let min_x = start[0].min(end[0]);
                let min_y = start[1].min(end[1]);
                let max_x = start[0].max(end[0]);
                let max_y = start[1].max(end[1]);
                ([min_x, min_y], [max_x, max_y])
            }
            AnnotationItem::Rectangle { min, max, .. }
            | AnnotationItem::Ellipse { min, max, .. }
            | AnnotationItem::Highlight { min, max, .. }
            | AnnotationItem::Obfuscate { min, max, .. } => {
                let min_x = min[0].min(max[0]);
                let min_y = min[1].min(max[1]);
                let max_x = min[0].max(max[0]);
                let max_y = min[1].max(max[1]);
                ([min_x, min_y], [max_x, max_y])
            }
            AnnotationItem::Text { pos, font_size, text, .. } => {
                let width = (text.len() as f32) * font_size * 0.65;
                let height = *font_size;
                ([pos[0], pos[1]], [pos[0] + width, pos[1] + height])
            }
            AnnotationItem::StepBadge { pos, radius, .. } => (
                [pos[0] - radius, pos[1] - radius],
                [pos[0] + radius, pos[1] + radius],
            ),
        }
    }

    pub fn translate(&mut self, delta: [f32; 2]) {
        match self {
            AnnotationItem::Freehand { points, .. } => {
                for p in points {
                    p[0] += delta[0];
                    p[1] += delta[1];
                }
            }
            AnnotationItem::Arrow { start, end, .. } => {
                start[0] += delta[0];
                start[1] += delta[1];
                end[0] += delta[0];
                end[1] += delta[1];
            }
            AnnotationItem::Rectangle { min, max, .. }
            | AnnotationItem::Ellipse { min, max, .. }
            | AnnotationItem::Highlight { min, max, .. }
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
    pub spans: Vec<Option<AnnotationSpan>>,
    pub next_badge_number: u32,
}

impl Scene {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            spans: Vec::new(),
            next_badge_number: 1,
        }
    }

    pub fn push_item(&mut self, item: AnnotationItem) {
        if let AnnotationItem::StepBadge { number, .. } = &item {
            self.next_badge_number = self.next_badge_number.max(number + 1);
        }
        self.items.push(item);
        self.spans.push(None);
    }

    pub fn push_item_with_span(&mut self, item: AnnotationItem, span: Option<AnnotationSpan>) {
        if let AnnotationItem::StepBadge { number, .. } = &item {
            self.next_badge_number = self.next_badge_number.max(number + 1);
        }
        self.items.push(item);
        self.spans.push(span);
    }

    pub fn remove_item(&mut self, idx: usize) -> Option<AnnotationItem> {
        if idx < self.items.len() {
            if idx < self.spans.len() {
                self.spans.remove(idx);
            }
            let removed = self.items.remove(idx);
            self.recalc_badge_number();
            Some(removed)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.spans.clear();
        self.next_badge_number = 1;
    }

    pub fn recalc_badge_number(&mut self) {
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

    /// Check if item at `idx` is visible at `time_ms`. None span means always visible.
    pub fn is_item_active_at(&self, idx: usize, time_ms: u64) -> bool {
        match self.spans.get(idx).and_then(|s| *s) {
            Some(span) => span.is_active_at(time_ms),
            None => true,
        }
    }
}

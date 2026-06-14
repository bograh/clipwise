use egui::{Color32, Stroke};

pub const BG_BASE: Color32 = Color32::from_rgb(18, 18, 22);
pub const BG_SURFACE: Color32 = Color32::from_rgb(26, 26, 32);
pub const BG_ELEVATED: Color32 = Color32::from_rgb(32, 32, 40);
pub const BG_HOVER: Color32 = Color32::from_rgb(40, 40, 50);
pub const BG_SELECTED: Color32 = Color32::from_rgb(44, 44, 58);
pub const BG_DETAIL: Color32 = Color32::from_rgb(22, 22, 28);
pub const BG_CARD: Color32 = Color32::from_rgb(30, 30, 38);

pub const BORDER_SUBTLE: Color32 = Color32::from_rgb(38, 38, 48);
pub const BORDER_ACTIVE: Color32 = Color32::from_rgb(58, 58, 72);

pub const ACCENT: Color32 = Color32::from_rgb(100, 140, 255);
pub const ACCENT_SUBTLE: Color32 = Color32::from_rgb(38, 50, 82);
pub const ACCENT_RED: Color32 = Color32::from_rgb(255, 85, 85);


pub const TEXT_BRIGHT: Color32 = Color32::from_rgb(240, 240, 245);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(195, 200, 210);
pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(130, 135, 148);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(88, 92, 108);

pub const KEY_BG: Color32 = Color32::from_rgb(38, 38, 48);
pub const KEY_BORDER: Color32 = Color32::from_rgb(48, 48, 60);

pub const PIN_COLOR: Color32 = Color32::from_rgb(255, 186, 75);

pub const CORNER_RADIUS: f32 = 8.0;
pub const CARD_RADIUS: f32 = 10.0;
pub const PILL_RADIUS: f32 = 6.0;

pub fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_BASE;
    visuals.window_fill = BG_BASE;
    visuals.extreme_bg_color = BG_BASE;
    visuals.code_bg_color = BG_ELEVATED;
    visuals.selection.bg_fill = ACCENT_SUBTLE;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, BORDER_ACTIVE);
    visuals.widgets.active.bg_fill = BG_SELECTED;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER_SUBTLE);
    visuals.override_text_color = Some(TEXT_PRIMARY);
    visuals.window_rounding = CORNER_RADIUS.into();
    visuals.menu_rounding = CORNER_RADIUS.into();
    ctx.set_visuals(visuals);
}
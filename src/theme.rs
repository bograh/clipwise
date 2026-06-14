pub const BG_PRIMARY: egui::Color32 = egui::Color32::from_rgb(28, 28, 30);
pub const BG_ELEVATED: egui::Color32 = egui::Color32::from_rgb(44, 44, 46);
pub const BG_SELECTED: egui::Color32 = egui::Color32::from_rgb(50, 50, 56);
pub const BG_HOVER: egui::Color32 = egui::Color32::from_rgb(38, 38, 44);
pub const BG_DETAIL: egui::Color32 = egui::Color32::from_rgb(34, 34, 36);
pub const SEPARATOR: egui::Color32 = egui::Color32::from_rgb(52, 52, 58);
pub const ICON_COLOR: egui::Color32 = egui::Color32::from_rgb(85, 85, 98);
pub const KEY_BG: egui::Color32 = egui::Color32::from_rgb(50, 50, 56);
pub const ACCENT_BLUE: egui::Color32 = egui::Color32::from_rgb(10, 132, 255);
pub const ACCENT_RED: egui::Color32 = egui::Color32::from_rgb(255, 69, 58);
pub const TEXT_PRIMARY: egui::Color32 = egui::Color32::from_rgb(245, 245, 247);
pub const TEXT_SECONDARY: egui::Color32 = egui::Color32::from_rgb(155, 155, 162);
pub const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(108, 108, 115);
// pub const ACCENT_GOLD: egui::Color32 = egui::Color32::from_rgb(255, 214, 10);

pub fn setup_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG_PRIMARY;
    visuals.window_fill = BG_PRIMARY;
    visuals.extreme_bg_color = BG_PRIMARY;
    visuals.code_bg_color = BG_PRIMARY;
    visuals.selection.bg_fill = BG_SELECTED;
    visuals.widgets.inactive.bg_fill = BG_ELEVATED;
    visuals.widgets.hovered.bg_fill = BG_HOVER;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, SEPARATOR);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    visuals.override_text_color = Some(TEXT_PRIMARY);
    ctx.set_visuals(visuals);
}

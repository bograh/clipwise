use crate::app::{sort_items, ClipwiseApp};
use crate::clipboard::ClipboardItem;
use crate::theme::*;
use chrono::{DateTime, Utc};
use egui::{
    Align, Color32, Frame, Id, Key, Layout, Margin, Pos2, Rect, RichText, Rounding, ScrollArea,
    Sense, Stroke, Vec2,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

enum RowAction {
    ConfirmDelete(String),
    CancelDelete,
    Select(usize),
}

fn relative_time(dt: &DateTime<Utc>, now: DateTime<Utc>) -> String {
    let dur = now.signed_duration_since(*dt);
    let secs = dur.num_seconds();
    let mins = dur.num_minutes();
    let hours = dur.num_hours();
    let days = dur.num_days();

    if secs < 60 {
        "Just now".to_string()
    } else if mins < 60 {
        format!("{}m ago", mins)
    } else if hours < 24 {
        format!("{}h ago", hours)
    } else if days == 1 {
        "Yesterday".to_string()
    } else if days < 7 {
        format!("{}d ago", days)
    } else {
        dt.format("%b %-d").to_string()
    }
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    let mut char_count = 0;
    let mut end_byte = content.len();
    for (i, _) in content.char_indices() {
        if char_count >= max_chars {
            end_byte = i;
            break;
        }
        char_count += 1;
    }
    let result: String = content[..end_byte]
        .chars()
        .map(|c| if c == '\n' || c == '\r' || c == '\t' { ' ' } else { c })
        .collect();
    if end_byte < content.len() {
        result + "…"
    } else {
        result
    }
}

fn compute_filtered(items: &[ClipboardItem], query: &str) -> Vec<usize> {
    if query.is_empty() {
        return (0..items.len()).collect();
    }
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(usize, i64)> = items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            matcher
                .fuzzy_match(&item.content, query)
                .map(|score| (i, score))
        })
        .collect();
    scored.sort_by(|a, b| match (items[a.0].pinned, items[b.0].pinned) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.1.cmp(&a.1),
    });
    scored.into_iter().map(|(i, _)| i).collect()
}

fn date_group(dt: &DateTime<Utc>, now: &DateTime<Utc>) -> &'static str {
    let days = now.signed_duration_since(*dt).num_days().max(0);
    match days {
        0 => "Today",
        1 => "Yesterday",
        2..=6 => "Earlier this week",
        7..=30 => "This month",
        _ => "Older",
    }
}

fn draw_clip_icon(ui: &mut egui::Ui, size: f32, is_pinned: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let r = size * 0.35;

    if is_pinned {
        let pin_pts = [
            Pos2::new(center.x, center.y - r),
            Pos2::new(center.x + r * 0.45, center.y - r * 0.3),
            Pos2::new(center.x + r * 0.45, center.y + r * 0.5),
            Pos2::new(center.x, center.y + r),
            Pos2::new(center.x - r * 0.45, center.y + r * 0.5),
            Pos2::new(center.x - r * 0.45, center.y - r * 0.3),
        ];
        let path = egui::epaint::PathShape::convex_polygon(
            pin_pts.to_vec(),
            Color32::from_rgb(60, 80, 160),
            Stroke::NONE,
        );
        painter.add(path);
        painter.circle_filled(center, r * 0.35, PIN_COLOR);
    } else {
        let doc_w = size * 0.40;
        let doc_h = size * 0.52;
        let doc_rect = Rect::from_center_size(center, Vec2::new(doc_w, doc_h));
        painter.rect_filled(doc_rect, 3.0, Color32::from_rgba_unmultiplied(100, 130, 255, 25));
        painter.rect_stroke(doc_rect, 3.0, Stroke::new(1.0, Color32::from_rgba_unmultiplied(100, 140, 255, 50)));
        let lx1 = doc_rect.min.x + 3.0;
        let lx2 = doc_rect.max.x - 3.0;
        for frac in [0.30_f32, 0.52, 0.74] {
            let ly = doc_rect.min.y + doc_h * frac;
            painter.line_segment(
                [Pos2::new(lx1, ly), Pos2::new(lx2, ly)],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(140, 160, 220, 60)),
            );
        }
    }
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        ui.label(RichText::new(text).size(10.5).color(TEXT_MUTED).strong());
    });
    ui.add_space(4.0);
}

fn render_item_row(
    ui: &mut egui::Ui,
    item: &ClipboardItem,
    disp_idx: usize,
    is_selected: bool,
    in_confirm_delete: bool,
) -> Option<RowAction> {
    let row_height = if in_confirm_delete { 68.0_f32 } else { 46.0_f32 };
    let row_min = ui.cursor().min;
    let row_width = ui.available_width();
    let row_rect = Rect::from_min_size(row_min, Vec2::new(row_width, row_height));

    let is_hovered = ui.rect_contains_pointer(row_rect);
    let bg = if in_confirm_delete {
        Color32::from_rgba_unmultiplied(255, 85, 85, 20)
    } else if is_selected {
        BG_SELECTED
    } else if is_hovered {
        BG_HOVER
    } else {
        Color32::TRANSPARENT
    };

    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(row_rect, 6.0, bg);
    }

    if is_selected {
        let bar_rect = Rect::from_min_size(
            Pos2::new(row_min.x + 4.0, row_min.y + 6.0),
            Vec2::new(3.0, row_height - 12.0),
        );
        ui.painter().rect_filled(bar_rect, 2.0, ACCENT);
    }

    let mut action: Option<RowAction> = None;

    ui.allocate_ui_with_layout(
        Vec2::new(row_width, row_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            if in_confirm_delete {
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Delete this clip?").color(TEXT_BRIGHT).size(12.5));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Cancel").size(11.5).color(TEXT_SECONDARY),
                                )
                                .fill(BG_ELEVATED)
                                .rounding(Rounding::same(5.0))
                                .stroke(Stroke::new(1.0, BORDER_SUBTLE)),
                            )
                            .clicked()
                        {
                            action = Some(RowAction::CancelDelete);
                        }
                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Delete").size(11.5).color(Color32::WHITE),
                                )
                                .fill(ACCENT_RED)
                                .rounding(Rounding::same(5.0)),
                            )
                            .clicked()
                        {
                            action = Some(RowAction::ConfirmDelete(item.id.clone()));
                        }
                    });
                });
            } else {
                ui.add_space(14.0);
                draw_clip_icon(ui, 26.0, item.pinned);
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    let preview = truncate_content(&item.content, 50);
                    ui.label(RichText::new(preview).color(TEXT_PRIMARY).size(12.5));
                    ui.add_space(2.0);
                    let time = relative_time(&item.copied_at, Utc::now());
                    ui.label(RichText::new(time).size(10.0).color(TEXT_MUTED));
                });
            }
        },
    );

    if !in_confirm_delete {
        let row_response = ui.interact(row_rect, Id::new("row_click").with(disp_idx), Sense::click());
        if row_response.clicked() {
            action = Some(RowAction::Select(disp_idx));
        }
        if row_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    action
}

fn metadata_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_space(2.0);
        ui.label(RichText::new(key).size(11.0).color(TEXT_MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(value).size(11.0).color(TEXT_SECONDARY));
        });
    });
    ui.add_space(6.0);
}

fn render_search_bar(ctx: &egui::Context, app: &mut ClipwiseApp) {
    egui::TopBottomPanel::top("search_bar")
        .exact_height(54.0)
        .frame(
            Frame::none()
                .fill(BG_SURFACE)
                .inner_margin(Margin::same(0.0))
                .stroke(Stroke::new(1.0, BORDER_SUBTLE)),
        )
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(12.0);

                let dismiss = ui.add(
                    egui::Button::new(RichText::new("✕").size(13.0).color(TEXT_MUTED))
                        .frame(false)
                        .rounding(Rounding::same(4.0)),
                );
                if dismiss.clicked() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    app.focus_requested = false;
                    app.confirm_delete = None;
                }
                if dismiss.hovered() {
                    ui.painter().rect_filled(
                        dismiss.rect,
                        4.0,
                        Color32::from_rgba_unmultiplied(255, 255, 255, 8),
                    );
                }

                ui.add_space(8.0);

                let search_width = (ui.available_width() - 130.0).max(100.0);

                Frame::none()
                    .fill(BG_ELEVATED)
                    .rounding(Rounding::same(8.0))
                    .inner_margin(Margin::symmetric(10.0, 6.0))
                    .stroke(Stroke::new(1.0, if app.search_query.is_empty() { BORDER_SUBTLE } else { Color32::from_rgb(60, 80, 160) }))
                    .show(ui, |ui| {
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.label(RichText::new("🔍").size(12.0));
                            ui.add_space(6.0);
                            let search_w = search_width - 60.0;
                            let resp = ui.add_sized(
                                Vec2::new(search_w, 20.0),
                                egui::TextEdit::singleline(&mut app.search_query)
                                    .hint_text("Search clips…")
                                    .frame(false)
                                    .text_color(TEXT_PRIMARY),
                            );
                            if !app.focus_requested {
                                resp.request_focus();
                                app.focus_requested = true;
                            }
                            if !app.search_query.is_empty() {
                                ui.add_space(4.0);
                                if ui
                                    .add(
                                        egui::Button::new(RichText::new("×").size(11.0).color(TEXT_MUTED))
                                            .frame(false),
                                    )
                                    .clicked()
                                {
                                    app.search_query.clear();
                                }
                            }
                        });
                    });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(10.0);
                    Frame::none()
                        .fill(BG_ELEVATED)
                        .rounding(Rounding::same(PILL_RADIUS))
                        .inner_margin(Margin::symmetric(8.0, 4.0))
                        .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("≡").size(10.0).color(TEXT_MUTED));
                                ui.add_space(3.0);
                                ui.label(RichText::new("All").size(11.0).color(TEXT_SECONDARY));
                                ui.add_space(2.0);
                                ui.label(RichText::new("▾").size(8.0).color(TEXT_MUTED));
                            });
                        });
                });
            });
        });
}

fn render_action_bar(ctx: &egui::Context, has_items: bool) {
    egui::TopBottomPanel::bottom("action_bar")
        .exact_height(36.0)
        .frame(
            Frame::none()
                .fill(BG_SURFACE)
                .inner_margin(Margin::symmetric(14.0, 0.0))
                .stroke(Stroke::new(1.0, BORDER_SUBTLE)),
        )
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("Clipwise").size(11.0).color(TEXT_MUTED).strong());

                if has_items {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.add_space(2.0);
                        key_badge(ui, "K", 3.0);
                        key_badge(ui, "Ctrl", 0.0);
                        ui.add_space(5.0);
                        ui.label(RichText::new("Show").size(10.5).color(TEXT_MUTED));

                        ui.add_space(14.0);

                        key_badge(ui, "↵", 3.0);
                        ui.add_space(5.0);
                        ui.label(RichText::new("Paste").size(10.5).color(TEXT_SECONDARY));

                        ui.add_space(14.0);

                        key_badge(ui, "⌫", 3.0);
                        ui.add_space(5.0);
                        ui.label(RichText::new("Delete").size(10.5).color(TEXT_MUTED));
                    });
                }
            });
        });
}

fn key_badge(ui: &mut egui::Ui, text: &str, right_space: f32) {
    Frame::none()
        .fill(KEY_BG)
        .rounding(Rounding::same(4.0))
        .inner_margin(Margin::symmetric(5.0, 2.0))
        .stroke(Stroke::new(1.0, KEY_BORDER))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.0).color(TEXT_SECONDARY).strong());
        });
    ui.add_space(right_space);
}

fn render_list_panel(ctx: &egui::Context, app: &mut ClipwiseApp) -> Option<RowAction> {
    let now = Utc::now();
    let filtered_items = compute_filtered(&app.items, &app.search_query);
    let mut ui_action: Option<RowAction> = None;

    egui::SidePanel::left("list_panel")
        .exact_width(270.0)
        .resizable(false)
        .frame(Frame::none().fill(BG_BASE))
        .show_separator_line(false)
        .show(ctx, |ui| {
            let pinned_indices: Vec<usize> = filtered_items
                .iter()
                .copied()
                .filter(|&i| app.items[i].pinned)
                .collect();
            let unpinned_indices: Vec<usize> = filtered_items
                .iter()
                .copied()
                .filter(|&i| !app.items[i].pinned)
                .collect();

            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(4.0);

                    if filtered_items.is_empty() {
                        ui.add_space(64.0);
                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            let msg = if app.search_query.is_empty() {
                                "No clips yet"
                            } else {
                                "No matching clips"
                            };
                            ui.label(RichText::new(msg).size(13.0).color(TEXT_MUTED));
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new("Copy something to get started")
                                    .size(11.0)
                                    .color(Color32::from_rgba_unmultiplied(255, 255, 255, 30)),
                            );
                        });
                        return;
                    }

                    if !pinned_indices.is_empty() {
                        section_label(ui, "PINNED");
                        for &item_idx in &pinned_indices {
                            let disp_idx = filtered_items
                                .iter()
                                .position(|&i| i == item_idx)
                                .unwrap_or(0);
                            let is_selected = disp_idx == app.selected_index;
                            let in_confirm = app
                                .confirm_delete
                                .as_deref()
                                .map_or(false, |id| id == app.items[item_idx].id);
                            if let Some(a) = render_item_row(
                                ui,
                                &app.items[item_idx],
                                disp_idx,
                                is_selected,
                                in_confirm,
                            ) {
                                ui_action = Some(a);
                            }
                        }
                    }

                    if !unpinned_indices.is_empty() {
                        let mut current_group = "";
                        for &item_idx in &unpinned_indices {
                            let group = date_group(&app.items[item_idx].copied_at, &now);
                            if group != current_group {
                                current_group = group;
                                section_label(ui, group);
                            }
                            let disp_idx = filtered_items
                                .iter()
                                .position(|&i| i == item_idx)
                                .unwrap_or(0);
                            let is_selected = disp_idx == app.selected_index;
                            let in_confirm = app
                                .confirm_delete
                                .as_deref()
                                .map_or(false, |id| id == app.items[item_idx].id);
                            if let Some(a) = render_item_row(
                                ui,
                                &app.items[item_idx],
                                disp_idx,
                                is_selected,
                                in_confirm,
                            ) {
                                ui_action = Some(a);
                            }
                        }
                    }
                });
        });

    ui_action
}

fn render_detail_panel(ctx: &egui::Context, selected_item: Option<&ClipboardItem>, now: DateTime<Utc>) {
    egui::CentralPanel::default()
        .frame(Frame::none().fill(BG_DETAIL))
        .show(ctx, |ui| {
            if let Some(item) = selected_item {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);

                        Frame::none()
                            .fill(BG_CARD)
                            .rounding(Rounding::same(CARD_RADIUS))
                            .inner_margin(Margin::symmetric(16.0, 14.0))
                            .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&item.content)
                                        .size(13.0)
                                        .color(TEXT_PRIMARY),
                                );
                            });

                        ui.add_space(12.0);

                        Frame::none()
                            .fill(BG_CARD)
                            .rounding(Rounding::same(CARD_RADIUS))
                            .inner_margin(Margin::symmetric(16.0, 12.0))
                            .stroke(Stroke::new(1.0, BORDER_SUBTLE))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Details")
                                        .size(10.5)
                                        .color(TEXT_MUTED)
                                        .strong(),
                                );
                                ui.add_space(8.0);

                                metadata_row(ui, "Type", "Text");

                                let char_count = item.content.chars().count();
                                metadata_row(ui, "Characters", &char_count.to_string());

                                let time_str = relative_time(&item.copied_at, now);
                                metadata_row(ui, "Copied", &time_str);

                                if item.pinned {
                                    metadata_row(ui, "Pinned", "Yes");
                                }
                            });

                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            ui.add_space(4.0);
                            key_badge(ui, "Enter", 3.0);
                            ui.label(RichText::new("Copy & close").size(10.5).color(TEXT_MUTED));
                            ui.add_space(12.0);
                            key_badge(ui, "Ctrl+D", 3.0);
                            ui.label(RichText::new("Pin/Unpin").size(10.5).color(TEXT_MUTED));
                            ui.add_space(12.0);
                            key_badge(ui, "Del", 3.0);
                            ui.label(RichText::new("Delete").size(10.5).color(TEXT_MUTED));
                        });
                    });
            } else {
                let rect = ui.max_rect();
                ui.vertical_centered(|ui| {
                    ui.add_space(rect.height() / 2.0 - 40.0);
                    ui.label(
                        RichText::new("Select a clip")
                            .size(14.0)
                            .color(Color32::from_rgba_unmultiplied(255, 255, 255, 25)),
                    );
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Use ↑↓ then Enter to paste")
                            .size(11.0)
                            .color(Color32::from_rgba_unmultiplied(255, 255, 255, 15)),
                    );
                });
            }
        });
}

pub fn render(ctx: &egui::Context, app: &mut ClipwiseApp) {
    let now = Utc::now();
    let mut filtered_items = compute_filtered(&app.items, &app.search_query);

    if !filtered_items.is_empty() && app.selected_index >= filtered_items.len() {
        app.selected_index = filtered_items.len().saturating_sub(1);
    }

    let mut do_escape = false;
    let mut do_enter = false;
    let mut do_up = false;
    let mut do_down = false;
    let mut do_ctrl_d = false;
    let mut do_delete = false;

    ctx.input(|i| {
        do_escape = i.key_pressed(Key::Escape);
        do_enter = i.key_pressed(Key::Enter);
        do_up = i.key_pressed(Key::ArrowUp);
        do_down = i.key_pressed(Key::ArrowDown);
        do_ctrl_d = i.key_pressed(Key::D) && i.modifiers.ctrl;
        do_delete = (i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
            && app.search_query.is_empty();
    });

    if do_escape {
        if !app.search_query.is_empty() {
            app.search_query.clear();
        } else {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            app.focus_requested = false;
            app.confirm_delete = None;
        }
    }

    if do_up && app.selected_index > 0 {
        app.selected_index -= 1;
    }
    if do_down && !filtered_items.is_empty() && app.selected_index + 1 < filtered_items.len() {
        app.selected_index += 1;
    }

    if do_enter {
        if let Some(&item_idx) = filtered_items.get(app.selected_index) {
            if let Some(item) = app.items.get(item_idx) {
                let content = item.content.clone();
                if let Ok(mut cb) = arboard::Clipboard::new() {
                    let _ = cb.set_text(content);
                }
            }
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        app.focus_requested = false;
        app.confirm_delete = None;
    }

    if do_ctrl_d {
        if let Some(&item_idx) = filtered_items.get(app.selected_index) {
            if let Some(item) = app.items.get_mut(item_idx) {
                item.pinned = !item.pinned;
            }
            sort_items(&mut app.items);
            let _ = app.storage.save_all(&app.items);
            filtered_items = compute_filtered(&app.items, &app.search_query);
        }
    }

    if do_delete {
        if let Some(&item_idx) = filtered_items.get(app.selected_index) {
            if let Some(item) = app.items.get(item_idx) {
                app.confirm_delete = Some(item.id.clone());
            }
        }
    }

    let selected_item: Option<ClipboardItem> = filtered_items
        .get(app.selected_index)
        .and_then(|&idx| app.items.get(idx))
        .cloned();

    let has_items = !app.items.is_empty();

    render_search_bar(ctx, app);
    render_action_bar(ctx, has_items);
    let list_action = render_list_panel(ctx, app);
    render_detail_panel(ctx, selected_item.as_ref(), now);

    match list_action {
        Some(RowAction::Select(disp_idx)) => {
            app.selected_index = disp_idx;
            app.confirm_delete = None;
        }
        Some(RowAction::ConfirmDelete(id)) => {
            app.items.retain(|i| i.id != id);
            app.confirm_delete = None;
            let _ = app.storage.delete_item_and_order(&id, &app.items);
        }
        Some(RowAction::CancelDelete) => {
            app.confirm_delete = None;
        }
        None => {}
    }
}
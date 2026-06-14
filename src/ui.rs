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
        format!("{} min ago", mins)
    } else if hours < 24 {
        format!("{} hr ago", hours)
    } else if days == 1 {
        "Yesterday".to_string()
    } else if days < 7 {
        format!("{} days ago", days)
    } else {
        dt.format("%b %-d, %Y").to_string()
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
        2..=6 => "This Week",
        7..=30 => "This Month",
        _ => "Older",
    }
}

fn draw_doc_icon(ui: &mut egui::Ui, size: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    let w = size * 0.50;
    let h = size * 0.62;
    let doc_rect = Rect::from_center_size(rect.center(), Vec2::new(w, h));
    painter.rect_filled(doc_rect, 2.0, ICON_COLOR);
    let line_color = Color32::from_rgba_unmultiplied(220, 220, 230, 55);
    let lx1 = doc_rect.min.x + 2.5;
    let lx2 = doc_rect.max.x - 2.5;
    for (i, frac) in [0.30_f32, 0.52, 0.74].iter().enumerate() {
        let ly = doc_rect.min.y + h * frac;
        let rx = if i == 2 { lx2 - 3.0 } else { lx2 };
        painter.line_segment(
            [Pos2::new(lx1, ly), Pos2::new(rx, ly)],
            Stroke::new(1.0, line_color),
        );
    }
}

fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        ui.label(RichText::new(text).size(11.0).color(TEXT_MUTED));
    });
    ui.add_space(3.0);
}

fn render_item_row(
    ui: &mut egui::Ui,
    item: &ClipboardItem,
    disp_idx: usize,
    is_selected: bool,
    in_confirm_delete: bool,
) -> Option<RowAction> {
    let row_height = if in_confirm_delete { 72.0_f32 } else { 42.0_f32 };
    let row_min = ui.cursor().min;
    let row_width = ui.available_width();
    let row_rect = Rect::from_min_size(row_min, Vec2::new(row_width, row_height));

    let is_hovered = ui.rect_contains_pointer(row_rect);
    let bg = if is_selected {
        BG_SELECTED
    } else if is_hovered {
        BG_HOVER
    } else {
        BG_PRIMARY
    };
    ui.painter().rect_filled(row_rect, 0.0, bg);

    if is_selected {
        ui.painter().rect_filled(
            Rect::from_min_size(row_min, Vec2::new(2.5, row_height)),
            0.0,
            ACCENT_BLUE,
        );
    }

    let mut action: Option<RowAction> = None;

    ui.allocate_ui_with_layout(
        Vec2::new(row_width, row_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            if in_confirm_delete {
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.add_space(8.0);
                    ui.label(RichText::new("Delete this item?").color(TEXT_PRIMARY).size(13.0));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Cancel").size(12.0).color(TEXT_SECONDARY),
                                )
                                .fill(BG_ELEVATED)
                                .rounding(Rounding::same(5.0)),
                            )
                            .clicked()
                        {
                            action = Some(RowAction::CancelDelete);
                        }
                        ui.add_space(4.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Delete").size(12.0).color(Color32::WHITE),
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
                ui.add_space(12.0);
                draw_doc_icon(ui, 28.0);
                ui.add_space(10.0);
                let preview = truncate_content(&item.content, 58);
                ui.label(RichText::new(preview).color(TEXT_PRIMARY).size(13.0));
            }
        },
    );

    if !in_confirm_delete {
        let row_response = ui.interact(row_rect, Id::new("row_click").with(disp_idx), Sense::click());
        if row_response.clicked() {
            action = Some(RowAction::Select(disp_idx));
        }
    }

    action
}

fn metadata_row(ui: &mut egui::Ui, key: &str, value: &str) {
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(key).size(12.0).color(TEXT_MUTED));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(4.0);
            ui.label(RichText::new(value).size(12.0).color(TEXT_PRIMARY));
        });
    });
    ui.add_space(4.0);
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

    let mut ui_action: Option<RowAction> = None;

    // ── Bottom action bar ────────────────────────────────────────────────────
    egui::TopBottomPanel::bottom("action_bar")
        .exact_height(40.0)
        .frame(Frame::none().fill(BG_PRIMARY).inner_margin(Margin::symmetric(14.0, 0.0)))
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(RichText::new("📋").size(12.0).color(TEXT_MUTED));
                ui.add_space(6.0);
                ui.label(RichText::new("Clipboard History").size(12.0).color(TEXT_MUTED));

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(2.0);
                    Frame::none()
                        .fill(KEY_BG)
                        .rounding(Rounding::same(4.0))
                        .inner_margin(Margin::symmetric(5.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("K").size(11.0).color(TEXT_SECONDARY));
                        });
                    Frame::none()
                        .fill(KEY_BG)
                        .rounding(Rounding::same(4.0))
                        .inner_margin(Margin::symmetric(5.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("Ctrl").size(11.0).color(TEXT_SECONDARY));
                        });
                    ui.add_space(3.0);
                    ui.label(RichText::new("Actions").size(12.0).color(TEXT_MUTED));

                    ui.add_space(14.0);

                    Frame::none()
                        .fill(KEY_BG)
                        .rounding(Rounding::same(4.0))
                        .inner_margin(Margin::symmetric(5.0, 2.0))
                        .show(ui, |ui| {
                            ui.label(RichText::new("↩").size(11.0).color(TEXT_SECONDARY));
                        });
                    ui.add_space(3.0);
                    ui.label(RichText::new("Paste").size(12.0).color(TEXT_PRIMARY));
                });
            });
        });

    // ── Top search + filter bar ──────────────────────────────────────────────
    egui::TopBottomPanel::top("search_bar")
        .exact_height(50.0)
        .frame(Frame::none().fill(BG_PRIMARY))
        .show_separator_line(true)
        .show(ctx, |ui| {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.add_space(10.0);

                // Back/dismiss arrow
                if ui
                    .add(
                        egui::Button::new(RichText::new("←").size(16.0).color(TEXT_MUTED))
                            .frame(false),
                    )
                    .clicked()
                {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
                    app.focus_requested = false;
                    app.confirm_delete = None;
                }

                ui.add_space(10.0);

                // Calculate search width leaving room for filter pill
                let filter_pill_width = 122.0_f32;
                let right_pad = 12.0_f32;
                let arrow_and_gap = 46.0_f32;
                let search_w =
                    (ui.available_width() - filter_pill_width - right_pad - arrow_and_gap)
                        .max(80.0);

                let resp = ui.add_sized(
                    Vec2::new(search_w, 32.0),
                    egui::TextEdit::singleline(&mut app.search_query)
                        .hint_text("Type to filter entries…")
                        .frame(false),
                );
                if !app.focus_requested {
                    resp.request_focus();
                    app.focus_requested = true;
                }

                ui.add_space(8.0);

                // All Types filter pill (visual)
                Frame::none()
                    .fill(BG_ELEVATED)
                    .rounding(Rounding::same(6.0))
                    .inner_margin(Margin::symmetric(8.0, 5.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("≡").size(11.0).color(TEXT_MUTED));
                            ui.add_space(4.0);
                            ui.label(RichText::new("All Types").size(12.0).color(TEXT_PRIMARY));
                            ui.add_space(4.0);
                            ui.label(RichText::new("▾").size(9.0).color(TEXT_MUTED));
                        });
                    });

                ui.add_space(10.0);
            });
        });

    // ── Left list panel ──────────────────────────────────────────────────────
    egui::SidePanel::left("list_panel")
        .exact_width(264.0)
        .resizable(false)
        .frame(Frame::none().fill(BG_PRIMARY))
        .show_separator_line(true)
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
                    if filtered_items.is_empty() {
                        ui.add_space(48.0);
                        ui.with_layout(Layout::top_down(Align::Center), |ui| {
                            let msg = if app.search_query.is_empty() {
                                "No clipboard history yet"
                            } else {
                                "No results"
                            };
                            ui.label(RichText::new(msg).size(13.0).color(TEXT_MUTED));
                        });
                        return;
                    }

                    // Pinned section
                    if !pinned_indices.is_empty() {
                        section_label(ui, "Pinned");
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

                    // Unpinned items, grouped by date
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

    // ── Right detail panel ───────────────────────────────────────────────────
    egui::CentralPanel::default()
        .frame(Frame::none().fill(BG_DETAIL))
        .show(ctx, |ui| {
            if let Some(ref item) = selected_item {
                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // Full content
                        Frame::none()
                            .inner_margin(Margin::symmetric(18.0, 16.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(&item.content).size(13.0).color(TEXT_PRIMARY),
                                );
                            });

                        // Information section separator
                        let sep_y = ui.cursor().min.y;
                        let sep_x1 = ui.min_rect().min.x;
                        let sep_x2 = ui.min_rect().max.x;
                        ui.painter().line_segment(
                            [Pos2::new(sep_x1, sep_y), Pos2::new(sep_x2, sep_y)],
                            Stroke::new(1.0, SEPARATOR),
                        );

                        // Metadata
                        Frame::none()
                            .inner_margin(Margin::symmetric(18.0, 12.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Information").size(11.0).color(TEXT_MUTED),
                                );
                                ui.add_space(8.0);

                                metadata_row(ui, "Type", "Text");

                                let char_count = item.content.chars().count();
                                metadata_row(ui, "Characters", &char_count.to_string());

                                let time_str = relative_time(&item.copied_at, now);
                                metadata_row(ui, "Copied", &time_str);
                            });
                    });
            } else {
                // Empty state
                let rect = ui.max_rect();
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No item selected",
                    egui::FontId::proportional(13.0),
                    TEXT_MUTED,
                );
            }
        });

    // ── Apply actions ────────────────────────────────────────────────────────
    match ui_action {
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

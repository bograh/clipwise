use crate::app::{sort_items, ClipwiseApp};
use crate::clipboard::ClipboardItem;
use crate::theme::*;
use chrono::{DateTime, Utc};
use egui::{Align, Frame, Key, Layout, Margin, Rect, RichText, Rounding, ScrollArea};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

enum RowAction {
    TogglePin(String),
    RequestDelete(String),
    ConfirmDelete(String),
    CancelDelete,
}

fn relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let dur = now.signed_duration_since(*dt);
    let secs = dur.num_seconds();
    let mins = dur.num_minutes();
    let hours = dur.num_hours();
    let days = dur.num_days();

    if secs < 60 {
        "Just now".to_string()
    } else if mins < 60 {
        format!("{} minutes ago", mins)
    } else if hours < 24 {
        format!("{} hours ago", hours)
    } else if days == 1 {
        "Yesterday".to_string()
    } else if days < 7 {
        format!("{} days ago", days)
    } else {
        dt.format("%b %-d, %Y").to_string()
    }
}

fn truncate_content(content: &str, max_chars: usize) -> String {
    let chars: Vec<char> = content.chars().collect();
    if chars.len() > max_chars {
        chars[..max_chars].iter().collect::<String>() + "…"
    } else {
        content.replace('\n', " ").replace('\t', " ")
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

fn render_item_row(
    ui: &mut egui::Ui,
    item: &ClipboardItem,
    is_selected: bool,
    in_confirm_delete: bool,
) -> Option<RowAction> {
    let row_height = if in_confirm_delete { 80.0_f32 } else { 56.0_f32 };
    let mut action: Option<RowAction> = None;

    let row_min = ui.cursor().min;
    let row_width = ui.available_width();
    let row_rect = Rect::from_min_size(row_min, egui::vec2(row_width, row_height));

    let is_hovered = ui.rect_contains_pointer(row_rect);
    let bg = if is_selected {
        BG_SELECTED
    } else if is_hovered {
        BG_ELEVATED
    } else {
        BG_PRIMARY
    };

    ui.painter().rect_filled(row_rect, 0.0, bg);

    if is_selected {
        ui.painter().rect_filled(
            Rect::from_min_size(row_min, egui::vec2(2.0, row_height)),
            0.0,
            ACCENT_BLUE,
        );
    }

    ui.allocate_ui_with_layout(
        egui::vec2(row_width, row_height),
        Layout::left_to_right(Align::Center),
        |ui| {
            if in_confirm_delete {
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.add_space(10.0);
                    ui.label(
                        RichText::new("Delete this item?")
                            .color(TEXT_PRIMARY)
                            .size(13.0),
                    );
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(RichText::new("Cancel").size(12.0))
                                    .fill(BG_ELEVATED),
                            )
                            .clicked()
                        {
                            action = Some(RowAction::CancelDelete);
                        }
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Delete")
                                        .color(egui::Color32::WHITE)
                                        .size(12.0),
                                )
                                .fill(ACCENT_RED),
                            )
                            .clicked()
                        {
                            action = Some(RowAction::ConfirmDelete(item.id.clone()));
                        }
                    });
                });
            } else {
                ui.add_space(8.0);

                // Pin button
                let pin_char = if item.pinned { "★" } else { "☆" };
                let pin_color = if item.pinned { ACCENT_GOLD } else { TEXT_MUTED };
                if ui
                    .add_sized(
                        egui::vec2(28.0, 28.0),
                        egui::Button::new(RichText::new(pin_char).color(pin_color).size(16.0))
                            .frame(false),
                    )
                    .clicked()
                {
                    action = Some(RowAction::TogglePin(item.id.clone()));
                }

                ui.add_space(4.0);

                // Content + timestamp
                let trash_space = 44.0;
                let content_width = (ui.available_width() - trash_space).max(0.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(content_width, row_height),
                    Layout::top_down(Align::LEFT),
                    |ui| {
                        ui.add_space(10.0);
                        let preview = truncate_content(&item.content, 80);
                        ui.label(RichText::new(preview).color(TEXT_PRIMARY).size(14.0));
                        ui.label(
                            RichText::new(relative_time(&item.copied_at))
                                .color(TEXT_MUTED)
                                .size(11.0),
                        );
                    },
                );

                // Trash button
                if ui
                    .add_sized(
                        egui::vec2(32.0, 32.0),
                        egui::Button::new(RichText::new("🗑").size(14.0)).frame(false),
                    )
                    .clicked()
                {
                    action = Some(RowAction::RequestDelete(item.id.clone()));
                }
            }
        },
    );

    action
}

pub fn render(ctx: &egui::Context, app: &mut ClipwiseApp) {
    let filtered_items = compute_filtered(&app.items, &app.search_query);

    if !filtered_items.is_empty() && app.selected_index >= filtered_items.len() {
        app.selected_index = filtered_items.len().saturating_sub(1);
    }

    // Read all keyboard input at once
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
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
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
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    if do_ctrl_d {
        if let Some(&item_idx) = filtered_items.get(app.selected_index) {
            if let Some(item) = app.items.get_mut(item_idx) {
                item.pinned = !item.pinned;
            }
            sort_items(&mut app.items);
            let _ = app.storage.save_all(&app.items);
        }
    }

    if do_delete {
        if let Some(&item_idx) = filtered_items.get(app.selected_index) {
            if let Some(item) = app.items.get(item_idx) {
                app.confirm_delete = Some(item.id.clone());
            }
        }
    }

    // Recompute after any mutations
    let filtered_items = compute_filtered(&app.items, &app.search_query);

    let mut ui_action: Option<RowAction> = None;

    egui::CentralPanel::default()
        .frame(Frame::none().fill(BG_PRIMARY))
        .show(ctx, |ui| {
            ui.add_space(8.0);

            // Search bar
            Frame::none()
                .fill(BG_ELEVATED)
                .inner_margin(Margin::symmetric(12.0, 8.0))
                .rounding(Rounding::same(8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔍").size(16.0).color(TEXT_MUTED));
                        ui.add_space(4.0);
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut app.search_query)
                                .hint_text("Search clipboard history…")
                                .desired_width(f32::INFINITY)
                                .frame(false),
                        );
                        if !app.focus_requested {
                            resp.request_focus();
                            app.focus_requested = true;
                        }
                    });
                });

            ui.add_space(4.0);

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
                    if !pinned_indices.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new("PINNED").size(10.0).color(TEXT_MUTED));
                        ui.add_space(4.0);
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
                                is_selected,
                                in_confirm,
                            ) {
                                ui_action = Some(a);
                            }
                        }
                    }

                    if !unpinned_indices.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new("RECENT").size(10.0).color(TEXT_MUTED));
                        ui.add_space(4.0);
                        for &item_idx in &unpinned_indices {
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
                                is_selected,
                                in_confirm,
                            ) {
                                ui_action = Some(a);
                            }
                        }
                    }
                });
        });

    // Apply UI actions
    match ui_action {
        Some(RowAction::TogglePin(id)) => {
            if let Some(item) = app.items.iter_mut().find(|i| i.id == id) {
                item.pinned = !item.pinned;
            }
            sort_items(&mut app.items);
            let _ = app.storage.save_all(&app.items);
        }
        Some(RowAction::RequestDelete(id)) => {
            app.confirm_delete = Some(id);
        }
        Some(RowAction::ConfirmDelete(id)) => {
            app.items.retain(|i| i.id != id);
            app.confirm_delete = None;
            let _ = app.storage.save_all(&app.items);
        }
        Some(RowAction::CancelDelete) => {
            app.confirm_delete = None;
        }
        None => {}
    }
}

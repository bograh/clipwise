use crate::clipboard::ClipboardItem;
use crate::storage::Storage;
use crate::ui;
use chrono::Utc;
use crossbeam_channel::Receiver;

const MAX_UNPINNED: usize = 500;

pub struct ClipwiseApp {
    pub items: Vec<ClipboardItem>,
    pub search_query: String,
    pub storage: Storage,
    pub receiver: Receiver<ClipboardItem>,
    pub show_receiver: Receiver<()>,
    pub confirm_delete: Option<String>,
    pub selected_index: usize,
    pub focus_requested: bool,
}

impl ClipwiseApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        items: Vec<ClipboardItem>,
        storage: Storage,
        receiver: Receiver<ClipboardItem>,
        show_tx: crossbeam_channel::Sender<()>,
        show_receiver: Receiver<()>,
    ) -> Self {
        crate::theme::setup_visuals(&cc.egui_ctx);
        crate::ipc::start_listener(show_tx, cc.egui_ctx.clone());
        Self {
            items,
            search_query: String::new(),
            storage,
            receiver,
            show_receiver,
            confirm_delete: None,
            selected_index: 0,
            focus_requested: false,
        }
    }
}

pub fn sort_items(items: &mut Vec<ClipboardItem>) {
    items.sort_by(|a, b| match (a.pinned, b.pinned) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.copied_at.cmp(&a.copied_at),
    });
}

impl eframe::App for ClipwiseApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Drain show signals from the IPC listener.
        while let Ok(()) = self.show_receiver.try_recv() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            self.search_query.clear();
            self.focus_requested = false;
        }

        while let Ok(new_item) = self.receiver.try_recv() {
            if let Some(pos) = self.items.iter().position(|i| i.content == new_item.content) {
                // Dedup: bump timestamp and move to top. Only one item changed, so use
                // the targeted write instead of clearing and rewriting the entire tree.
                let mut existing = self.items.remove(pos);
                existing.copied_at = Utc::now();
                let id = existing.id.clone();
                self.items.insert(0, existing);
                sort_items(&mut self.items);
                if let Some(updated) = self.items.iter().find(|i| i.id == id) {
                    let _ = self.storage.save_item_and_order(updated, &self.items);
                }
            } else {
                self.items.insert(0, new_item);
                let unpinned_count = self.items.iter().filter(|i| !i.pinned).count();
                if unpinned_count > MAX_UNPINNED {
                    if let Some(pos) = self.items.iter().rposition(|i| !i.pinned) {
                        self.items.remove(pos);
                    }
                }
                sort_items(&mut self.items);
                let _ = self.storage.save_all(&self.items);
            }
        }

        ui::render(ctx, self);
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
}

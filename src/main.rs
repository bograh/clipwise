mod app;
mod clipboard;
mod ipc;
mod storage;
mod theme;
mod ui;

use app::ClipwiseApp;
use storage::Storage;

fn main() {
    // If a daemon is already running, signal it to show and exit.
    if ipc::try_signal_existing() {
        return;
    }

    let storage = Storage::open()
        .expect("Failed to open clipboard storage at ~/.local/share/clipwise/db");
    let items = storage.load_all().expect("Failed to load clipboard history");

    let (clip_tx, clip_rx) = crossbeam_channel::unbounded();
    clipboard::start_watcher(clip_tx);

    // show_tx goes to the IPC listener (started inside ClipwiseApp::new where
    // egui::Context is available); show_rx is drained in update().
    let (show_tx, show_rx) = crossbeam_channel::unbounded();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 520.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top()
            .with_visible(false),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Clipwise",
        options,
        Box::new(move |cc| {
            Ok(Box::new(ClipwiseApp::new(
                cc, items, storage, clip_rx, show_tx, show_rx,
            )))
        }),
    )
    .expect("Failed to launch Clipwise");
}

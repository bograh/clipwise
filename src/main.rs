mod app;
mod clipboard;
mod storage;
mod theme;
mod ui;

use app::ClipwiseApp;
use storage::Storage;

fn main() {
    let storage = Storage::open().expect("Failed to open clipboard storage at ~/.local/share/clipwise/db");
    let items = storage.load_all().expect("Failed to load clipboard history");

    let (tx, rx) = crossbeam_channel::unbounded();
    clipboard::start_watcher(tx);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([680.0, 520.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_always_on_top(),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        "Clipwise",
        options,
        Box::new(move |cc| Ok(Box::new(ClipwiseApp::new(cc, items, storage, rx)))),
    )
    .expect("Failed to launch Clipwise");
}

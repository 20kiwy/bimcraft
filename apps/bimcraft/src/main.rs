//! BimCraft desktop entry point (eframe/glow).
//!
//! The model autosaves into eframe storage (per-user app data) and can be
//! opened/saved as native `.bimcraft.json` files from the app bar.

use bimcraft_ui_egui::BimcraftApp;

const STORAGE_KEY: &str = "bimcraft_model_v2";

/// Thin eframe shell delegating to the shared UI crate.
struct Shell {
    inner: BimcraftApp,
}

impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.inner.ui(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &bimcraft_io::save_json(&self.inner.session.model));
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title("BimCraft").with_inner_size([1440.0, 900.0]).with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    eframe::run_native(
        "BimCraft",
        options,
        Box::new(|cc| {
            let inner = cc
                .storage
                .and_then(|s| eframe::get_value::<String>(s, STORAGE_KEY))
                .and_then(|json| bimcraft_io::load_json(&json).ok())
                .map(BimcraftApp::with_model)
                .unwrap_or_default();
            Ok(Box::new(Shell { inner }))
        }),
    )
}

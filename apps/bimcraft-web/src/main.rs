//! BimCraft browser edition entry point (eframe web runner on wasm32).
//!
//! The model autosaves into eframe web storage (localStorage), so a refresh
//! keeps your work; use the CLI to exchange native `.bimcraft.json` files.

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    // The browser edition only builds for wasm32; desktop users run the
    // `bimcraft` app instead. This stub keeps `cargo check` happy on native.
    eprintln!("bimcraft-web is the wasm browser edition; build with: trunk build (see README.md)");
    eprintln!("For the desktop app run: cargo run -p bimcraft");
}

#[cfg(target_arch = "wasm32")]
use bimcraft_ui_egui::BimcraftApp;

/// Thin eframe shell delegating to the shared UI crate (same as desktop).
#[cfg(target_arch = "wasm32")]
struct Shell {
    inner: BimcraftApp,
}

#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "bimcraft_model_v2";

#[cfg(target_arch = "wasm32")]
impl eframe::App for Shell {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.inner.ui(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, STORAGE_KEY, &bimcraft_io::save_json(&self.inner.session.model));
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    console_error_panic_hook::set_once();
    use wasm_bindgen::JsCast;
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async move {
        let document = web_sys::window().expect("no window").document().expect("no document");
        let canvas = document
            .get_element_by_id("bimcraft_canvas")
            .expect("missing #bimcraft_canvas")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("canvas is not a HtmlCanvasElement");
        let app_creator = Box::new(|cc: &eframe::CreationContext<'_>| {
            let inner = cc
                .storage
                .and_then(|s| eframe::get_value::<String>(s, STORAGE_KEY))
                .and_then(|json| bimcraft_io::load_json(&json).ok())
                .map(BimcraftApp::with_model)
                .unwrap_or_default();
            Ok(Box::new(Shell { inner }) as Box<dyn eframe::App>)
        });
        if let Err(e) = eframe::WebRunner::new().start(canvas, web_options, app_creator).await {
            log::error!("eframe web start failed: {e:?}");
        } else {
            // First frame is up: tell the shell to drop the loading overlay
            // (index.html waits for this event; it used to stay on screen
            // forever because nothing ever dispatched it).
            if let Some(win) = web_sys::window() {
                let event = web_sys::CustomEvent::new("bimcraft-ready").ok();
                if let Some(ev) = event {
                    let _ = win.dispatch_event(&ev);
                }
            }
        }
    });
}

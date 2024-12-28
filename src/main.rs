#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release
#![allow(rustdoc::missing_crate_level_docs)] // it's an example
                                             //
use eframe::egui;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Run the publisher in another task
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([320.0, 240.0]),
        ..Default::default()
    };
    let _ = eframe::run_native(
        "My Monitoring App",
        options,
        Box::new(|_cc| {
            // This gives us image support:
            Ok(Box::<MyApp>::default())
        }),
    );
    return Ok(());
}

struct MyApp {
    node: r2r::Node,
    node_map: std::collections::HashMap<String, Vec<String>>,
}

impl Default for MyApp {
    fn default() -> Self {
        let ctx = r2r::Context::create().unwrap();
        Self {
            node: r2r::Node::create(ctx, "node", "namespace").unwrap(),
            node_map: std::collections::HashMap::new(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Topic list");
            for (_name, _types) in self.node_map.clone() {
                ui.label(_name);
            }
            if ui.button("refresh").on_hover_text("refresh").clicked() {
                self.node_map = self.node.get_topic_names_and_types().unwrap();
            };
        });
    }
}

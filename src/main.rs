#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on Windows in release mode
#![allow(rustdoc::missing_crate_level_docs)] // Example

use eframe::egui;
use futures::stream::StreamExt;
use r2r::{Context, Node, QosProfile};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize the logger
    env_logger::init();

    let options = eframe::NativeOptions {
        ..Default::default()
    };

    let ctx = Context::create().unwrap();
    let node = Node::create(ctx, "sub_node", "namespace").unwrap();
    let arc_node = Arc::new(Mutex::new(node));

    let n = arc_node.clone();
    // tokio::task::spawn(async move { subscriber(n).await.unwrap() });

    let handle = tokio::task::spawn_blocking(move || loop {
        {
            arc_node
                .lock()
                .unwrap()
                .spin_once(std::time::Duration::from_millis(10));
        }
        std::thread::sleep(std::time::Duration::from_millis(100))
    });

    let _ = eframe::run_native(
        "My Monitoring App",
        options,
        Box::new(|_cc| Ok(Box::new(MyApp::default_with_channel(n)))),
    );

    handle.await.unwrap();
    Ok(())
}

async fn subscriber(arc_node: Arc<Mutex<r2r::Node>>) -> Result<(), r2r::Error> {
    let sub = arc_node
        .lock()
        .unwrap()
        .subscribe_untyped("/chatter", "std_msgs/msg/String", QosProfile::default())
        .unwrap();

    let _ = sub
        .for_each(|msg| async move {
            match msg {
                Ok(msg) => match serde_json::to_string_pretty(&msg) {
                    Ok(json) => println!("Received message: {}\n---\n", json),
                    Err(err) => eprintln!("Failed to serialize message: {}", err),
                },
                Err(err) => eprintln!("Failed to receive message: {}", err),
            }
        })
        .await;
    Ok(())
}

struct MyApp {
    node_map: HashMap<String, Vec<String>>,
    node: Arc<Mutex<Node>>,
    sub_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MyApp {
    fn default_with_channel(node: Arc<Mutex<Node>>) -> Self {
        Self {
            node,
            node_map: HashMap::new(),
            sub_handle: None,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Topic list");
            for (name, types) in &self.node_map {
                ui.label(name);
                ui.label(types.join(", "));
            }

            if ui
                .button("Refresh")
                .on_hover_text("Refresh topic list")
                .clicked()
            {
                match self.node.lock().unwrap().get_topic_names_and_types() {
                    Ok(topics) => {
                        self.node_map = topics;
                    }
                    Err(err) => {
                        eprintln!("Failed to refresh topic list: {:?}", err);
                    }
                }
            }

            if ui
                .button("Add Subscriber")
                .on_hover_text("Subscribe to /joy")
                .clicked()
            {
                match self.sub_handle.take() {
                    Some(handle) => {
                        handle.abort();
                    }
                    None => {}
                }
                let n = self.node.clone();

                self.sub_handle = Some(tokio::task::spawn(
                    async move { subscriber(n).await.unwrap() },
                ));
            }
        });
    }
}

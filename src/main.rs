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

async fn subscriber(
    arc_node: Arc<Mutex<r2r::Node>>,
    sender_channel: Arc<Mutex<tokio::sync::watch::Sender<String>>>,
    ctx: &egui::Context,
    topic: &str,
    msg_type: &str,
) -> Result<(), r2r::Error> {
    let sub = arc_node
        .lock()
        .unwrap()
        .subscribe_untyped(topic, msg_type, QosProfile::default())
        .unwrap();

    let _ = sub
        .for_each(|msg| {
            let s = sender_channel.clone();
            async move {
                match msg {
                    Ok(msg) => match serde_json::to_string_pretty(&msg) {
                        Ok(json) => {
                            println!("received msg \n{}\n---\n", json);
                            s.lock().unwrap().send(json).unwrap();
                            ctx.request_repaint();
                        }
                        Err(err) => eprintln!("Failed to serialize message: {}", err),
                    },
                    Err(err) => eprintln!("Failed to receive message: {}", err),
                }
            }
        })
        .await;
    Ok(())
}

struct MyApp {
    node_map: HashMap<String, Vec<String>>,
    node: Arc<Mutex<Node>>,
    sub_handle: Option<tokio::task::JoinHandle<()>>,
    sender_channel: Arc<Mutex<tokio::sync::watch::Sender<String>>>,
    receiver_channel: tokio::sync::watch::Receiver<String>,
    clicked_topic: String,
    clicked_type: String,
}

impl MyApp {
    fn default_with_channel(node: Arc<Mutex<Node>>) -> Self {
        let (sender, receiver) = tokio::sync::watch::channel("no data received".to_string());
        Self {
            node,
            sender_channel: Arc::new(Mutex::new(sender)),
            receiver_channel: receiver,
            node_map: HashMap::new(),
            sub_handle: None,
            clicked_topic: "".to_string(),
            clicked_type: "".to_string(),
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::bottom("Status Bar").show(ctx, |ui| {
            use egui::special_emojis::GITHUB;
            ui.hyperlink_to(
                format!("{GITHUB} source on GitHub"),
                "https://github.com/pusin819/topic_monitor",
            );
        });

        egui::SidePanel::left("Topic List").show(ctx, |ui| {
            ui.heading("Topic List");

            for (name, types) in &self.node_map {
                if ui
                    .radio(self.clicked_topic == *name, name)
                    .on_hover_text(format!("{:?}", types))
                    .clicked()
                {
                    self.clicked_topic = name.to_string();
                    self.clicked_type = types[0].to_string();

                    // Delete the previous subscriber
                    self.sender_channel
                        .lock()
                        .unwrap()
                        .send("".to_string())
                        .unwrap();

                    match self.sub_handle.take() {
                        Some(handle) => {
                            handle.abort();
                        }
                        None => {}
                    }
                    let n = self.node.clone();
                    let s = self.sender_channel.clone();
                    let c = ctx.clone();
                    let topic = self.clicked_topic.clone();
                    let msg_type = self.clicked_type.clone();

                    self.sub_handle = Some(tokio::task::spawn(async move {
                        subscriber(n, s, &c, &topic, &msg_type).await.unwrap()
                    }));
                }
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

                ctx.request_repaint();
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(self.receiver_channel.borrow_and_update().to_string());
        });
    }
}

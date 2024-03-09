// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use openconnect_core::{
    config::{ConfigBuilder, LogLevel},
    Connectable, VpnClient,
};
use std::sync::{mpsc::Sender, Arc, Mutex};

static mut RESULT_SENDER: Option<Mutex<Sender<UIEvent>>> = None;
static mut CLIENT: Option<Arc<VpnClient>> = None;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command(rename_all = "snake_case")]
fn connect(server: String, username: String, password: String) {
    let tx = unsafe { RESULT_SENDER.as_ref().unwrap().lock().unwrap() };
    let _ = tx.send(UIEvent::Connect {
        server,
        username,
        password,
    });
}

#[tauri::command]
fn disconnect() {
    println!("disconnecting");
    let tx = unsafe { RESULT_SENDER.as_ref().unwrap().lock().unwrap() };
    let _ = tx.send(UIEvent::Disconnect);
}

#[derive(Clone, serde::Serialize)]
enum UIEvent {
    Connect {
        server: String,
        username: String,
        password: String,
    },
    Disconnect,
}

fn main() {
    sudo::escalate_if_needed().unwrap();

    let (tx, rx) = std::sync::mpsc::channel::<UIEvent>();

    unsafe {
        RESULT_SENDER = Some(Mutex::new(tx));
    }

    std::thread::spawn(move || loop {
        match rx.recv() {
            Ok(UIEvent::Connect {
                server,
                username,
                password,
            }) => {
                unsafe {
                    CLIENT = Some(
                        VpnClient::new(
                            ConfigBuilder::default()
                                .username(&username)
                                .password(&password)
                                .server(&server)
                                .loglevel(LogLevel::Info)
                                .build()
                                .unwrap(),
                            openconnect_core::events::EventHandlers::default(),
                        )
                        .unwrap(),
                    )
                };

                let client = unsafe { CLIENT.as_ref() };
                if let Some(client) = client {
                    std::thread::spawn(move || {
                        client.connect().unwrap();
                    });
                }
            }
            Ok(UIEvent::Disconnect) => {
                let client = unsafe { CLIENT.as_ref() };
                if let Some(client) = client {
                    client.disconnect();
                    unsafe {
                        CLIENT = None;
                    }
                }
                println!("disconnecting");
            }
            Err(_) => {
                println!("UI thread has been disconnected");
                break;
            }
        }
    });

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, connect, disconnect])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

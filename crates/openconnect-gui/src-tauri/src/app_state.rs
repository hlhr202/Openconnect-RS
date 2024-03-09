use std::sync::Arc;

use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    Connectable, VpnClient,
};

use crate::VpnEvent;

#[derive(Clone, serde::Serialize)]
pub enum VpnCommand {
    Connect {
        server: String,
        username: String,
        password: String,
    },
    Disconnect,
    Destory,
}

pub struct AppState {
    tx: std::sync::mpsc::Sender<VpnCommand>,
    client: Arc<VpnClient>,
}

impl AppState {
    pub fn new(
        event_tx: std::sync::mpsc::Sender<VpnEvent>,
    ) -> (Self, std::sync::mpsc::Receiver<VpnCommand>) {
        let (tx, rx) = std::sync::mpsc::channel::<VpnCommand>();
        let config = ConfigBuilder::default()
            .loglevel(LogLevel::Info)
            .build()
            .unwrap();

        let event_handlers =
            EventHandlers::default().with_handle_connection_state_change(move |state| {
                let result = event_tx.send(VpnEvent::Status(state));
                if let Err(e) = result {
                    eprintln!("Error while emitting event: {:?}", e);
                }
            });

        (
            Self {
                tx,
                client: VpnClient::new(config, event_handlers).unwrap(),
            },
            rx,
        )
    }

    pub fn send(&self, command: VpnCommand) {
        self.tx.send(command).unwrap();
    }

    pub fn run(&self, rx: std::sync::mpsc::Receiver<VpnCommand>) {
        let client = self.client.clone();
        std::thread::spawn(move || 'event_loop: loop {
            match rx.recv() {
                Ok(VpnCommand::Connect {
                    server,
                    username,
                    password,
                }) => {
                    let entrypoint = EntrypointBuilder::new()
                        .server(&server)
                        .username(&username)
                        .password(&password)
                        .enable_udp(true)
                        .build()
                        .unwrap();

                    let client = client.clone();
                    std::thread::spawn(move || {
                        client.connect(entrypoint).unwrap();
                    });
                }
                Ok(VpnCommand::Disconnect) => {
                    client.disconnect();
                }
                Ok(VpnCommand::Destory) => {
                    client.disconnect();
                    break 'event_loop;
                }
                Err(_) => break 'event_loop,
            }
        });
    }
}

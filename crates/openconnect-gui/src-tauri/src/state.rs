use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    Connectable, Status, VpnClient,
};
use std::sync::Arc;
use tauri::Manager;

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

#[derive(Debug, Clone)]
pub enum VpnEvent {
    Status(StatusPayload),
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct StatusPayload {
    status: String,
    message: Option<String>,
}

impl From<Status> for StatusPayload {
    fn from(status: Status) -> Self {
        let (status, message) = match status {
            Status::Initialized => ("initialized".to_string(), None),
            Status::Connecting => ("connecting".to_string(), None),
            Status::Connected => ("connected".to_string(), None),
            Status::Disconnecting => ("disconnecting".to_string(), None),
            Status::Disconnected => ("disconnected".to_string(), None),
            Status::Error(err) => ("error".to_string(), Some(err.to_string())),
        };

        Self { status, message }
    }
}

pub struct AppState {
    pub(crate) tx: std::sync::mpsc::Sender<VpnCommand>,
}

impl AppState {
    pub fn handle(app: &mut tauri::App) -> std::thread::JoinHandle<()> {
        let (event_tx, event_rx) = std::sync::mpsc::channel::<VpnEvent>();
        let app_state = AppState::create_and_run_loop(event_tx);
        app.manage(app_state);

        let handle = app.app_handle();

        std::thread::spawn(move || loop {
            if let Ok(event) = event_rx.recv() {
                match event {
                    VpnEvent::Status(status) => {
                        let result = handle.emit_all("vpnStatus", Some(status));
                        if let Err(e) = result {
                            eprintln!("Error while emitting event: {:?}", e);
                        }
                    }
                }
            }
        })
    }

    pub fn create_and_run_loop(event_tx: std::sync::mpsc::Sender<VpnEvent>) -> Self {
        let (tx, rx) = std::sync::mpsc::channel::<VpnCommand>();

        let mut client: Option<Arc<VpnClient>> = None;

        std::thread::spawn(move || 'event_loop: loop {
            match rx.recv() {
                Ok(VpnCommand::Connect {
                    server,
                    username,
                    password,
                }) => {
                    let config = ConfigBuilder::default()
                        .loglevel(LogLevel::Info)
                        .build()
                        .unwrap();

                    let entrypoint = EntrypointBuilder::new()
                        .server(&server)
                        .username(&username)
                        .password(&password)
                        .enable_udp(true)
                        .build()
                        .unwrap();

                    let event_tx = event_tx.clone();

                    let event_handlers = EventHandlers::default()
                        .with_handle_connection_state_change(move |state| {
                            let result = event_tx.send(VpnEvent::Status(state.into()));
                            if let Err(e) = result {
                                eprintln!("Error while emitting event: {:?}", e);
                            }
                        });

                    client = Some(VpnClient::new(config, event_handlers).unwrap());

                    let client = client.clone();

                    std::thread::spawn(move || {
                        if let Some(client) = client {
                            client.connect(entrypoint).unwrap();
                        }
                    });
                }
                Ok(VpnCommand::Disconnect) => {
                    let this_client = client.clone();
                    if let Some(client) = this_client {
                        client.disconnect();
                    }
                }
                Ok(VpnCommand::Destory) => {
                    let this_client = client.clone();
                    if let Some(client) = this_client {
                        client.disconnect();
                    }
                    break 'event_loop;
                }
                Err(_) => break 'event_loop,
            }
        });

        Self { tx }
    }

    pub fn send(&self, command: VpnCommand) -> Result<(), std::sync::mpsc::SendError<VpnCommand>> {
        self.tx.send(command)
    }
}

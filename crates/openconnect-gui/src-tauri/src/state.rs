use curl::easy::List;
use openconnect_core::{
    config::{ConfigBuilder, EntrypointBuilder, LogLevel},
    events::EventHandlers,
    token::{Token, TokenMode},
    Connectable, Status, VpnClient,
};
use openidconnect::CsrfToken;
use reqwest::{
    header::{HeaderMap, AUTHORIZATION},
    Body, ClientBuilder,
};
use std::{str::FromStr, sync::Arc};
use tauri::Manager;
use url::Url;

use crate::oidc::OpenID;

#[derive(Clone, serde::Serialize)]
pub enum VpnCommand {
    Connect {
        server: String,
        username: String,
        password: String,
    },
    ConnectOpenID {
        server: String,
    },
    Disconnect,
    Destory,
    GetState,
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
        let mut csrf_state: Option<CsrfToken> = None;

        std::thread::spawn(move || 'event_loop: loop {
            match rx.recv() {
                Ok(VpnCommand::ConnectOpenID { server }) => {
                    let openid = OpenID::new(
                        env::var("OIDC_ISSUER").unwrap(),
                        env::var("OIDC_REDIRECT_URI").unwrap(),
                        env::var("OIDC_CLIENT_ID").unwrap(),
                        Some(env::var("OIDC_CLIENT_SECRET")),
                    )
                    .unwrap();

                    let (authorize_url, req_state, _) = openid.auth_request().unwrap();
                    csrf_state = Some(req_state.clone());
                    open::that(authorize_url.to_string()).unwrap();
                    let (code, callback_state) = openid.wait_for_callback().unwrap();

                    if req_state.secret() != callback_state.secret() {
                        eprintln!("Invalid state");
                        continue;
                    }

                    let token = openid.exchange_token(code).unwrap();
                    println!("Token: {:?}", token);
                    let cookie = obtain_cookie_by_oidc_token(&server, &token).unwrap();
                    println!("Cookie: {:?}", cookie);

                    let config = ConfigBuilder::default()
                        .loglevel(LogLevel::Info)
                        .build()
                        .unwrap();

                    let entrypoint = EntrypointBuilder::new()
                        .token(Token {
                            mode: TokenMode::OIDC,
                            value: Some(token),
                        })
                        .server(&server)
                        .cookie(&cookie)
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
                Ok(VpnCommand::GetState) => {
                    if let Some(client) = client.clone() {
                        let state = client.get_state();
                        let result = event_tx.send(VpnEvent::Status(state.into()));
                        if let Err(e) = result {
                            eprintln!("Error while emitting event: {:?}", e);
                        }
                    }
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

fn obtain_cookie_by_oidc_token(url: &str, token: &str) -> Option<String> {
    println!("url: {:?}", url);
    println!("token: {:?}", token);

    let res = ureq::post(url)
        .set("Authorization", &format!("Bearer={}", token))
        .call();
    if res.is_err() {
        eprintln!(
            "Error while sending request: {:?}",
            res.err().unwrap().to_string()
        );
        return None;
    }
    let text = res.unwrap().into_string().unwrap();

    // let client = reqwest::blocking::ClientBuilder::new()
    //     .danger_accept_invalid_certs(true)
    //     .default_headers(HeaderMap::new())
    //     .connection_verbose(true)
    //     .build()
    //     .unwrap();

    // let req_builder = client
    //     .post(Url::from_str(url).unwrap())
    //     .header(AUTHORIZATION, format!("Bearer={}", token));

    // let res = req_builder.send();
    // if let Err(e) = res {
    //     eprintln!("Error while sending request: {:?}", e);
    //     return None;
    // }
    // let res = res.ok()?;
    // if !res.status().is_success() {
    //     eprintln!(
    //         "Failed to obtain cookie from server. Error code: {:?}",
    //         res.status()
    //     );
    //     return None;
    // }
    // let text = res.text().ok()?;
    // println!("response: {:?}", text);

    None
}

use crate::{JsonRequest, JsonResponse};
use colored::Colorize;
use futures::SinkExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio_serde::{formats::SymmetricalJson, Framed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

#[cfg(not(target_os = "windows"))]
use tokio::net::{
    unix::{OwnedReadHalf, OwnedWriteHalf},
    UnixListener, UnixStream,
};

#[cfg(target_os = "windows")]
use tokio::{
    io::{ReadHalf, WriteHalf},
    net::windows::named_pipe::{ClientOptions, NamedPipeClient, NamedPipeServer, ServerOptions},
};

#[derive(Debug, Error)]
pub enum SockError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No valid connection")]
    NoValidConnection,
}

pub fn get_sock() -> PathBuf {
    let tmp = Path::new("/tmp").to_path_buf();
    tmp.join("openconnect-rs.sock")
}

pub fn exit_when_socket_exists() {
    if get_sock().exists() {
        eprintln!("{}","\nSocket already exists. You may have a connected VPN session or a stale socket file. You may solve by:".red());
        eprintln!(
            "{}",
            "1. Stopping the connection by sending stop command.".red()
        );
        eprintln!(
            "2. Manually deleting the socket file which located at: {}",
            get_sock().display().to_string().red()
        );
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "windows"))]
pub type FramedWriter<T> =
    Framed<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>, T, T, SymmetricalJson<T>>;

#[cfg(not(target_os = "windows"))]
pub type FramedReader<T> =
    Framed<FramedRead<OwnedReadHalf, LengthDelimitedCodec>, T, T, SymmetricalJson<T>>;

#[cfg(target_os = "windows")]
pub type FramedWriter<T, R> =
    Framed<FramedWrite<WriteHalf<R>, LengthDelimitedCodec>, T, T, SymmetricalJson<T>>;

#[cfg(target_os = "windows")]
pub type FramedReader<T, R> =
    Framed<FramedRead<ReadHalf<R>, LengthDelimitedCodec>, T, T, SymmetricalJson<T>>;

#[cfg(not(target_os = "windows"))]
pub fn get_framed_writer<T: Sized>(write_half: OwnedWriteHalf) -> FramedWriter<T> {
    let length_delimited = FramedWrite::new(write_half, LengthDelimitedCodec::new());
    let codec = SymmetricalJson::<T>::default();
    tokio_serde::SymmetricallyFramed::new(length_delimited, codec)
}

#[cfg(not(target_os = "windows"))]
pub fn get_framed_reader<T: Sized>(read_half: OwnedReadHalf) -> FramedReader<T> {
    let length_delimited = FramedRead::new(read_half, LengthDelimitedCodec::new());
    let codec = SymmetricalJson::<T>::default();
    tokio_serde::SymmetricallyFramed::new(length_delimited, codec)
}

#[cfg(target_os = "windows")]
pub fn get_framed_writer<T: Sized, R: tokio::io::AsyncWrite>(
    write_half: WriteHalf<R>,
) -> FramedWriter<T, R> {
    let length_delimited = FramedWrite::new(write_half, LengthDelimitedCodec::new());
    let codec = SymmetricalJson::<T>::default();
    tokio_serde::SymmetricallyFramed::new(length_delimited, codec)
}

#[cfg(target_os = "windows")]
pub fn get_framed_reader<T: Sized, R: tokio::io::AsyncRead>(
    read_half: ReadHalf<R>,
) -> FramedReader<T, R> {
    let length_delimited = FramedRead::new(read_half, LengthDelimitedCodec::new());
    let codec = SymmetricalJson::<T>::default();
    tokio_serde::SymmetricallyFramed::new(length_delimited, codec)
}

pub struct UnixDomainServer {
    #[cfg(not(target_os = "windows"))]
    pub listener: UnixListener,

    #[cfg(target_os = "windows")]
    pub listener: Mutex<Option<NamedPipeServer>>,
}

impl UnixDomainServer {
    pub fn bind() -> Result<Self, SockError> {
        #[cfg(not(target_os = "windows"))]
        {
            let listener = UnixListener::bind(get_sock())?;
            let listener = listener.into_std()?;
            listener.set_nonblocking(true)?;
            let listener = UnixListener::from_std(listener)?;
        }

        #[cfg(target_os = "windows")]
        let listener = Mutex::new(Some(ServerOptions::new().create(get_sock())?));

        Ok(UnixDomainServer { listener })
    }

    #[cfg(not(target_os = "windows"))]
    pub async fn accept<R, W>(&self) -> std::io::Result<(FramedReader<R>, FramedWriter<W>)> {
        let (stream, _) = self.listener.accept().await?;
        let (read, write) = stream.into_split();
        Ok((get_framed_reader(read), get_framed_writer(write)))
    }

    #[cfg(target_os = "windows")]
    pub async fn accept<R, W>(
        &self,
    ) -> std::io::Result<(
        FramedReader<R, NamedPipeServer>,
        FramedWriter<W, NamedPipeServer>,
    )> {
        let mut server = self.listener.lock().await;
        let server = (*server).take();
        if let Some(server) = server {
            server.connect().await?;
            let (read, write) = tokio::io::split(server);
            Ok((get_framed_reader(read), get_framed_writer(write)))
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "NamedPipeServer is None",
            ))
        }
    }
}

impl Drop for UnixDomainServer {
    fn drop(&mut self) {
        // There's no way to return a useful error here
        std::fs::remove_file(get_sock()).expect("Failed to remove socket file");
    }
}

pub struct UnixDomainClient {
    #[cfg(not(target_os = "windows"))]
    framed_writer: FramedWriter<JsonRequest>,

    #[cfg(not(target_os = "windows"))]
    pub framed_reader: FramedReader<JsonResponse>,

    #[cfg(target_os = "windows")]
    framed_writer: FramedWriter<JsonRequest, NamedPipeClient>,

    #[cfg(target_os = "windows")]
    pub framed_reader: FramedReader<JsonResponse, NamedPipeClient>,
}

impl UnixDomainClient {
    pub async fn connect() -> Result<Self, SockError> {
        let sock = get_sock();
        if !sock.exists() {
            return Err(SockError::NoValidConnection);
        }

        #[cfg(not(target_os = "windows"))]
        {
            let stream = UnixStream::connect(sock).await?;
            let (read, write) = stream.into_split();

            let framed_writer = get_framed_writer(write);
            let framed_reader = get_framed_reader(read);

            Ok(UnixDomainClient {
                framed_writer,
                framed_reader,
            })
        }

        #[cfg(target_os = "windows")]
        {
            let stream = ClientOptions::new().open(get_sock())?;
            let (read, write) = tokio::io::split(stream);
            let framed_writer = get_framed_writer(write);
            let framed_reader = get_framed_reader(read);

            Ok(UnixDomainClient {
                framed_writer,
                framed_reader,
            })
        }
    }

    pub async fn send(&mut self, command: JsonRequest) -> Result<(), SockError> {
        self.framed_writer.send(command).await?;
        Ok(())
    }
}

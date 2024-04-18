use crate::{JsonRequest, JsonResponse};
use colored::Colorize;
use futures::SinkExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::net::{
    unix::{OwnedReadHalf, OwnedWriteHalf},
    UnixListener, UnixStream,
};
use tokio_serde::{formats::SymmetricalJson, Framed};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

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

pub type FramedWriter<T> =
    Framed<FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>, T, T, SymmetricalJson<T>>;
pub type FramedReader<T> =
    Framed<FramedRead<OwnedReadHalf, LengthDelimitedCodec>, T, T, SymmetricalJson<T>>;

pub fn get_framed_writer<T: Sized>(write_half: OwnedWriteHalf) -> FramedWriter<T> {
    let length_delimited = FramedWrite::new(write_half, LengthDelimitedCodec::new());
    let codec = SymmetricalJson::<T>::default();
    tokio_serde::SymmetricallyFramed::new(length_delimited, codec)
}

pub fn get_framed_reader<T: Sized>(read_half: OwnedReadHalf) -> FramedReader<T> {
    let length_delimited = FramedRead::new(read_half, LengthDelimitedCodec::new());
    let codec = SymmetricalJson::<T>::default();
    tokio_serde::SymmetricallyFramed::new(length_delimited, codec)
}

pub struct UnixDomainServer {
    pub listener: UnixListener,
}

impl UnixDomainServer {
    pub fn bind() -> Result<Self, SockError> {
        let listener = UnixListener::bind(get_sock())?;
        let listener = listener.into_std()?;
        listener.set_nonblocking(true)?;
        let listener = UnixListener::from_std(listener)?;
        Ok(UnixDomainServer { listener })
    }
}

impl Drop for UnixDomainServer {
    fn drop(&mut self) {
        // There's no way to return a useful error here
        std::fs::remove_file(get_sock()).expect("Failed to remove socket file");
    }
}

pub struct UnixDomainClient {
    framed_writer: FramedWriter<JsonRequest>,
    pub framed_reader: FramedReader<JsonResponse>,
}

impl UnixDomainClient {
    pub async fn connect() -> Result<Self, SockError> {
        let sock = get_sock();
        if !sock.exists() {
            return Err(SockError::NoValidConnection);
        }
        let stream = UnixStream::connect(sock).await?;
        let (read, write) = stream.into_split();
        let framed_writer = get_framed_writer(write);
        let framed_reader = get_framed_reader(read);

        Ok(UnixDomainClient {
            framed_writer,
            framed_reader,
        })
    }

    pub async fn send(&mut self, command: JsonRequest) -> Result<(), SockError> {
        self.framed_writer.send(command).await?;
        Ok(())
    }
}

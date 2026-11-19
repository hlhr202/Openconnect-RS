use crate::{JsonRequest, JsonResponse};
use colored::Colorize;
use futures::SinkExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tokio::net::{
    UnixListener, UnixStream,
    unix::{OwnedReadHalf, OwnedWriteHalf},
};
use tokio_serde::{Framed, formats::SymmetricalJson};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

#[derive(Debug, Error)]
pub enum SockError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("No valid connection")]
    NoValidConnection,
}

/// Determine the effective owning UID/GID for the daemon socket.
/// When invoked through `sudo`, preserve the invoking user's UID/GID so `status` and `stop`
/// work without re-escalating while keeping socket permissions strictly owner-only (`0600`).
pub fn invoking_uid_gid() -> (u32, u32) {
    if let (Ok(uid_s), Ok(gid_s)) = (std::env::var("SUDO_UID"), std::env::var("SUDO_GID"))
        && let (Ok(uid), Ok(gid)) = (uid_s.parse::<u32>(), gid_s.parse::<u32>())
    {
        return (uid, gid);
    }
    // SAFETY: `geteuid` and `getegid` are always safe per POSIX.
    let uid = unsafe { libc::geteuid() };
    // SAFETY: `getegid` is always safe per POSIX.
    let gid = unsafe { libc::getegid() };
    (uid, gid)
}

pub fn get_sock_dir() -> PathBuf {
    let (uid, _) = invoking_uid_gid();
    PathBuf::from(format!("/tmp/openconnect-rs-{}", uid))
}

pub fn get_sock() -> PathBuf {
    get_sock_dir().join("openconnect-rs.sock")
}

pub fn exit_when_socket_exists() {
    let sock = get_sock();
    if sock.exists() {
        eprintln!(
            "{}",
            "\nSocket already exists. You may have a connected VPN session or a stale socket file. You may solve by:"
                .red()
        );
        eprintln!(
            "{}",
            "1. Stopping the connection by sending stop command.".red()
        );
        eprintln!(
            "2. Manually deleting the socket file which located at: {}",
            sock.display().to_string().red()
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

fn chown_path(path: &Path, uid: u32, gid: u32) {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;
    if let Ok(cpath) = CString::new(path.as_os_str().as_bytes()) {
        // SAFETY: `cpath` is a valid NUL-terminated path string.
        let _ = unsafe { libc::chown(cpath.as_ptr(), uid, gid) };
    }
}

pub struct UnixDomainServer {
    pub listener: UnixListener,
    pub allowed_uid: u32,
    path: PathBuf,
}

impl UnixDomainServer {
    pub fn bind() -> Result<Self, SockError> {
        let dir = get_sock_dir();
        let (owner_uid, owner_gid) = invoking_uid_gid();

        std::fs::create_dir_all(&dir)?;
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))?;
        chown_path(&dir, owner_uid, owner_gid);

        let path = get_sock();
        let listener = UnixListener::bind(&path)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        chown_path(&path, owner_uid, owner_gid);

        let listener = listener.into_std()?;
        listener.set_nonblocking(true)?;
        let listener = UnixListener::from_std(listener)?;
        Ok(UnixDomainServer {
            listener,
            allowed_uid: owner_uid,
            path,
        })
    }
}

impl Drop for UnixDomainServer {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.path) {
            tracing::warn!(
                "Failed to remove socket file {}: {}",
                self.path.display(),
                err
            );
        }
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

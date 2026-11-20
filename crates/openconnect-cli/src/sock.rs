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

fn path_to_cstring(path: &Path) -> std::io::Result<std::ffi::CString> {
    use std::os::unix::ffi::OsStrExt;
    std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Socket path contains interior NUL byte",
        )
    })
}

/// Safely create or open the socket directory without following symlinks (`O_NOFOLLOW | O_DIRECTORY`),
/// verify via `fstat` that it is owned by root or `owner_uid`, and lock down mode `0700` via `fchmod`/`fchown`.
fn ensure_secure_sock_dir(dir: &Path, owner_uid: u32, owner_gid: u32) -> Result<(), SockError> {
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    let cdir = path_to_cstring(dir)?;

    // SAFETY: `cdir` is a valid NUL-terminated path; `mkdir` never follows a trailing symlink.
    let mkdir_rc = unsafe { libc::mkdir(cdir.as_ptr(), 0o700) };
    if mkdir_rc < 0 {
        let err = std::io::Error::last_os_error();
        if err.kind() != std::io::ErrorKind::AlreadyExists {
            return Err(SockError::Io(err));
        }
    }

    let open_flags = libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_DIRECTORY | libc::O_CLOEXEC;
    // SAFETY: `cdir` is a valid NUL-terminated path and `O_NOFOLLOW | O_DIRECTORY` refuses symlinks.
    let raw_fd = unsafe { libc::open(cdir.as_ptr(), open_flags) };
    if raw_fd < 0 {
        return Err(SockError::Io(std::io::Error::last_os_error()));
    }
    // SAFETY: `raw_fd` is a newly opened non-negative file descriptor exclusively owned here.
    let dir_fd = unsafe { OwnedFd::from_raw_fd(raw_fd) };

    let mut st = std::mem::MaybeUninit::<libc::stat>::zeroed();
    // SAFETY: `dir_fd` is a valid open descriptor and `st.as_mut_ptr()` is writable `libc::stat`.
    let fstat_rc = unsafe { libc::fstat(dir_fd.as_raw_fd(), st.as_mut_ptr()) };
    if fstat_rc < 0 {
        return Err(SockError::Io(std::io::Error::last_os_error()));
    }
    // SAFETY: `fstat` succeeded and fully initialized `st`.
    let st = unsafe { st.assume_init() };

    if (st.st_mode & libc::S_IFMT) != libc::S_IFDIR {
        return Err(SockError::Io(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Socket path {} is not a directory", dir.display()),
        )));
    }

    if st.st_uid != 0 && st.st_uid != owner_uid {
        return Err(SockError::Io(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!(
                "Refusing untrusted socket directory {} owned by uid {} (expected 0 or {})",
                dir.display(),
                st.st_uid,
                owner_uid
            ),
        )));
    }

    // SAFETY: `dir_fd` refers to the verified directory inode; `fchmod` cannot follow symlinks.
    let chmod_rc = unsafe { libc::fchmod(dir_fd.as_raw_fd(), 0o700) };
    if chmod_rc < 0 {
        return Err(SockError::Io(std::io::Error::last_os_error()));
    }

    if st.st_uid != owner_uid || st.st_gid != owner_gid {
        // SAFETY: `dir_fd` refers to the verified directory inode; `fchown` cannot follow symlinks.
        let chown_rc = unsafe { libc::fchown(dir_fd.as_raw_fd(), owner_uid, owner_gid) };
        if chown_rc < 0 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EPERM) {
                return Err(SockError::Io(err));
            }
        }
    }

    Ok(())
}

fn lchown_socket(path: &Path, uid: u32, gid: u32) -> std::io::Result<()> {
    let cpath = path_to_cstring(path)?;
    // SAFETY: `cpath` is a valid NUL-terminated path; `lchown` does not follow symlinks.
    let rc = unsafe { libc::lchown(cpath.as_ptr(), uid, gid) };
    if rc < 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() != Some(libc::EPERM) {
            return Err(err);
        }
    }
    Ok(())
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
        Self::bind_in_dir(&dir, owner_uid, owner_gid)
    }

    pub(crate) fn bind_in_dir(
        dir: &Path,
        owner_uid: u32,
        owner_gid: u32,
    ) -> Result<Self, SockError> {
        ensure_secure_sock_dir(dir, owner_uid, owner_gid)?;

        let path = dir.join("openconnect-rs.sock");
        let listener = UnixListener::bind(&path)?;

        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            return Err(SockError::Io(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Bound socket path unexpectedly resolved to a symlink",
            )));
        }

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
        lchown_socket(&path, owner_uid, owner_gid)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_bind_in_dir_creates_0700_dir_and_rejects_symlink_attack() {
        let tmp = tempfile::tempdir().unwrap();
        let (uid, gid) = invoking_uid_gid();

        // 1. Valid socket directory gets locked down to 0700 with 0600 socket.
        let valid_dir = tmp.path().join("sock-dir");
        {
            let server = UnixDomainServer::bind_in_dir(&valid_dir, uid, gid).unwrap();
            let dir_mode = std::fs::symlink_metadata(&valid_dir)
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            let sock_mode = std::fs::symlink_metadata(&server.path)
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(dir_mode, 0o700);
            assert_eq!(sock_mode, 0o600);
        }
        assert!(!valid_dir.join("openconnect-rs.sock").exists());

        // 2. Pre-created symlink directory MUST be rejected without mutating target permissions.
        let victim_target = tmp.path().join("victim-target");
        std::fs::create_dir(&victim_target).unwrap();
        std::fs::set_permissions(&victim_target, std::fs::Permissions::from_mode(0o755)).unwrap();

        let symlink_dir = tmp.path().join("symlink-sock-dir");
        std::os::unix::fs::symlink(&victim_target, &symlink_dir).unwrap();

        let res = UnixDomainServer::bind_in_dir(&symlink_dir, uid, gid);
        assert!(res.is_err(), "Symlinked socket directory must be rejected");

        let victim_mode = std::fs::symlink_metadata(&victim_target)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(
            victim_mode, 0o755,
            "Symlink target permissions must never be modified"
        );
    }
}

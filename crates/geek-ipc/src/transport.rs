//! The local endpoint: a Unix socket, or a named pipe on Windows.

use std::io;

use tokio::io::{AsyncRead, AsyncWrite};

/// Where the helper listens.
#[cfg(target_os = "linux")]
pub const ENDPOINT: &str = "/run/geekvpn-helper.sock";
#[cfg(target_os = "macos")]
pub const ENDPOINT: &str = "/var/run/geekvpn-helper.sock";
#[cfg(windows)]
pub const ENDPOINT: &str = r"\\.\pipe\geekvpn-helper";
#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub const ENDPOINT: &str = "/var/run/geekvpn-helper.sock";

pub trait Stream: AsyncRead + AsyncWrite + Unpin + Send + 'static {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send + 'static> Stream for T {}

#[cfg(unix)]
pub struct Listener(tokio::net::UnixListener);

#[cfg(unix)]
pub fn bind(path: &str) -> io::Result<Listener> {
    use std::os::unix::fs::PermissionsExt;
    // A socket left by a previous run: nobody can be listening on it, or
    // the service manager would not have started us.
    let _ = std::fs::remove_file(path);
    let l = tokio::net::UnixListener::bind(path)?;
    // Every local user may connect (see the crate docs).
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666))?;
    Ok(Listener(l))
}

#[cfg(unix)]
impl Listener {
    pub async fn accept(&mut self) -> io::Result<Box<dyn Stream>> {
        let (s, _) = self.0.accept().await?;
        Ok(Box::new(s))
    }
}

#[cfg(unix)]
pub async fn connect(path: &str) -> io::Result<Box<dyn Stream>> {
    Ok(Box::new(tokio::net::UnixStream::connect(path).await?))
}

#[cfg(windows)]
pub struct Listener {
    name: String,
    next: tokio::net::windows::named_pipe::NamedPipeServer,
}

#[cfg(windows)]
mod pipe {
    use std::io;

    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;

    /// SYSTEM and Administrators: full control; signed-in users: read and
    /// write. The default for a pipe made by SYSTEM lets users only read.
    const SDDL: &str = "D:(A;;GA;;;SY)(A;;GA;;;BA)(A;;GRGW;;;AU)";

    pub fn create(name: &str, first: bool) -> io::Result<NamedPipeServer> {
        let wide: Vec<u16> = SDDL.encode_utf16().chain(Some(0)).collect();
        let mut sd = std::ptr::null_mut();
        // SAFETY: `wide` is NUL-terminated; `sd` receives a LocalAlloc'd
        // descriptor, freed below.
        let ok = unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(wide.as_ptr(), 1, &mut sd, std::ptr::null_mut()) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        let mut sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: sd,
            bInheritHandle: 0,
        };
        // SAFETY: `sa` and the descriptor it points at live until the call
        // returns; the pipe copies what it needs.
        let server = unsafe {
            ServerOptions::new()
                // Refuse to start if someone else already owns the name, so
                // no user process can pose as the helper.
                .first_pipe_instance(first)
                .reject_remote_clients(true)
                .create_with_security_attributes_raw(name, &mut sa as *mut _ as *mut _)
        };
        // SAFETY: `sd` came from the call above and is not used again.
        unsafe { LocalFree(sd as _) };
        server
    }
}

#[cfg(windows)]
pub fn bind(name: &str) -> io::Result<Listener> {
    Ok(Listener { name: name.to_string(), next: pipe::create(name, true)? })
}

#[cfg(windows)]
impl Listener {
    pub async fn accept(&mut self) -> io::Result<Box<dyn Stream>> {
        self.next.connect().await?;
        let fresh = pipe::create(&self.name, false)?;
        Ok(Box::new(std::mem::replace(&mut self.next, fresh)))
    }
}

#[cfg(windows)]
pub async fn connect(name: &str) -> io::Result<Box<dyn Stream>> {
    use tokio::net::windows::named_pipe::ClientOptions;
    const ERROR_PIPE_BUSY: i32 = 231;
    for _ in 0..20 {
        match ClientOptions::new().open(name) {
            Ok(c) => return Ok(Box::new(c)),
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await
            }
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(io::ErrorKind::TimedOut, "helper pipe busy"))
}

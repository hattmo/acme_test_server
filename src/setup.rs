use std::{
    ffi::c_void,
    fmt::Display,
    io::{self, Result as IoResult},
    net::SocketAddr as IpSocketAddr,
    os::fd::{FromRawFd, IntoRawFd},
    pin::Pin,
    task::{Context, Poll},
};

use axum::serve::Listener;
use libc::{AF_INET, AF_INET6, AF_UNIX, SO_DOMAIN, SOL_SOCKET, c_int, getsockopt, socklen_t};
use tokio::{
    io::{AsyncRead, AsyncWrite, ReadBuf},
    net::{TcpListener, TcpStream, UnixListener, UnixStream},
};

pub enum GenericListener {
    Tcp(TcpListener),
    Unix(UnixListener),
}

pub enum GenericStream {
    Tcp(TcpStream),
    Unix(UnixStream),
}

#[derive(Debug)]
pub enum GenericAddr {
    Tcp(IpSocketAddr),
    Unix,
}

impl Display for GenericAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GenericAddr::Tcp(socket_addr) => socket_addr.fmt(f),
            GenericAddr::Unix => write!(f, "unix"),
        }
    }
}

impl AsyncRead for GenericStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        match Pin::into_inner(self) {
            GenericStream::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_read(cx, buf),
            GenericStream::Unix(unix_stream) => Pin::new(unix_stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for GenericStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<IoResult<usize>> {
        match Pin::into_inner(self) {
            GenericStream::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_write(cx, buf),
            GenericStream::Unix(unix_stream) => Pin::new(unix_stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        match Pin::into_inner(self) {
            GenericStream::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_flush(cx),
            GenericStream::Unix(unix_stream) => Pin::new(unix_stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        match Pin::into_inner(self) {
            GenericStream::Tcp(tcp_stream) => Pin::new(tcp_stream).poll_shutdown(cx),
            GenericStream::Unix(unix_stream) => Pin::new(unix_stream).poll_shutdown(cx),
        }
    }
}

impl Listener for GenericListener {
    type Io = GenericStream;

    type Addr = GenericAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self {
            GenericListener::Tcp(tcp_listener) => {
                let (sock, addr) = tcp_listener.accept().await;
                (GenericStream::Tcp(sock), GenericAddr::Tcp(addr))
            }
            GenericListener::Unix(unix_listener) => {
                let (sock, _) = unix_listener.accept().await;
                (GenericStream::Unix(sock), GenericAddr::Unix)
            }
        }
    }

    fn local_addr(&self) -> tokio::io::Result<Self::Addr> {
        match self {
            GenericListener::Tcp(tcp_listener) => {
                tcp_listener.local_addr().map(|addr| GenericAddr::Tcp(addr))
            }
            GenericListener::Unix(unix_listener) => {
                unix_listener.local_addr().map(|_| GenericAddr::Unix)
            }
        }
    }
}

pub struct ServerSockets {
    pub web: GenericListener,
    pub c2: GenericListener,
}

pub async fn setup_sockets() -> IoResult<ServerSockets> {
    Ok(
        if let Ok(Ok(i)) = std::env::var("LISTEN_FDS").and_then(|fds| Ok(fds.parse::<u8>()))
            && i == 2
        {
            ServerSockets {
                web: get_sd_socket(3)?,
                c2: get_sd_socket(4)?,
            }
        } else {
            let web = GenericListener::Tcp(TcpListener::bind("0.0.0.0:80").await?);
            let c2 = GenericListener::Tcp(TcpListener::bind("0.0.0.0:7777").await?);
            ServerSockets { web, c2 }
        },
    )
}

fn get_sd_socket(fd: impl IntoRawFd + Copy) -> IoResult<GenericListener> {
    let mut opt_val: c_int = 0;
    let mut opt_len = size_of_val(&opt_val) as socklen_t;
    if unsafe {
        getsockopt(
            fd.into_raw_fd(),
            SOL_SOCKET,
            SO_DOMAIN,
            std::ptr::from_mut(&mut opt_val) as *mut c_void,
            std::ptr::from_mut(&mut opt_len),
        )
    } != 0
    {
        return Err(io::Error::other("Unknown Type"));
    };
    match opt_val {
        AF_UNIX => {
            let listener =
                unsafe { std::os::unix::net::UnixListener::from_raw_fd(fd.into_raw_fd()) };
            listener.set_nonblocking(true)?;
            Ok(GenericListener::Unix(UnixListener::from_std(listener)?))
        }
        AF_INET | AF_INET6 => {
            let listener = unsafe { std::net::TcpListener::from_raw_fd(fd.into_raw_fd()) };
            listener.set_nonblocking(true)?;
            Ok(GenericListener::Tcp(TcpListener::from_std(listener)?))
        }
        _ => Err(io::Error::other("Unknown Type")),
    }
}

use async_native_tls::TlsStream;
use smol::net::TcpStream;
use smol::io::{AsyncRead, AsyncWrite};
use std::pin::Pin;
use std::task::{Poll, Context};
use std::marker::Unpin;
use std::io::Error as IoError;

#[derive(Debug)]
pub(super) enum ConnectType {
    Tls(TlsStream<TcpStream>),
    Normal(TcpStream),
}

impl Unpin for ConnectType {}

impl AsyncRead for ConnectType {
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<Result<usize, IoError>> {
        match self.get_mut() {
            Self::Tls(tls_stream) => Pin::new(tls_stream).poll_read(cx, buf),
            Self::Normal(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for ConnectType {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, IoError>> {
        match self.get_mut() {
            Self::Tls(tls_stream) => Pin::new(tls_stream).poll_write(cx, buf),
            Self::Normal(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
        match self.get_mut() {
            Self::Tls(tls_stream) => Pin::new(tls_stream).poll_flush(cx),
            Self::Normal(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), IoError>> {
    
        match self.get_mut() {
            Self::Tls(tls_stream) => Pin::new(tls_stream).poll_close(cx),
            Self::Normal(stream) => Pin::new(stream).poll_close(cx),
        }
    }
}

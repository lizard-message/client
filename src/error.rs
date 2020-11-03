use async_native_tls::Error as TlsError;
use protocol::send_to_server::decode::Error as DecodeError;
use std::io::Error as IoError;
use std::net::AddrParseError;
use std::string::FromUtf8Error;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("server addr parse error `{0}`")]
    Addr(#[from] AddrParseError),

    #[error("io error `{0}`")]
    Io(#[from] IoError),

    #[error("parse error `{0}`")]
    Parse(#[from] DecodeError),

    #[error("tls error `{0}`")]
    Tls(#[from] TlsError),

    #[error("hand shake error, because `{0}`")]
    HandShake(HandShakeError),

    #[error("convert utf8 error, because `{0}`")]
    Utf8(#[from] FromUtf8Error),
}

#[derive(Debug, Error)]
pub enum HandShakeError {
    #[error("tcp close")]
    ConnectClose,

    #[error("handshake message parse error")]
    Parse,

    #[error("client not select push or pull")]
    ClientPushOrPull,

    #[error("server not select push or pull")]
    ServerPushOrPull,
}

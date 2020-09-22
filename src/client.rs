use anyhow::Error;
use async_native_tls::{TlsConnector, TlsStream};
use smol::net::TcpStream;
use std::net::{AddrParseError, SocketAddr};
use smol::io::{AsyncWriteExt, AsyncReadExt};
use protocol::send_to_server::decode::Decode;

#[derive(Debug)]
enum ConnectType {
    Tls(TlsStream<TcpStream>),
    Normal(TcpStream),
}

#[derive(Debug)]
pub struct Builder<'a> {
    host: &'a str,
    port: u16,
    tls_option: Option<&'a str>,
}

impl<'a> Builder<'a> {
    pub fn new(host: &'a str, port: u16) -> Self {
        Self {
            host,
            port,
            tls_option: None,
        }
    }

    pub fn set_tls_domain(mut self, domain: &'a str) -> Self {
        self.tls_option = Some(domain);
        self
    }

    pub async fn connect(mut self) -> Result<Client, Error> {
        let addr = SocketAddr::new(self.host.parse()?, self.port);

        let mut connect = TcpStream::connect(addr).await?;

        let mut buff = [0u8; 1024];
        let mut decode = Decode::new(1024);

        loop {
            let size = connect.read(&mut buff).await?;

            if size == 0 {
                break;
            } else {
                decode.set_buff(&buff[..]);


            }
        }
    }
}

#[derive(Debug)]
pub struct Client {
    stream: ConnectType,
}

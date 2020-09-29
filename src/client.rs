use super::error::{Error, HandShakeError};
use async_native_tls::connect;
use protocol::send_to_server::{
    decode::{Decode, Message},
    encode::ClientConfig,
};
use protocol::state::Support;
use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::net::TcpStream;
use std::default::Default;
use std::net::SocketAddr;
use super::connect_type::ConnectType;
use super::mode::Mode;

#[derive(Debug)]
pub struct Builder<'a> {
    host: &'a str,
    port: u16,
    tls_option: Option<&'a str>,
    support: u16,
}

impl<'a> Builder<'a> {
    pub fn new(host: &'a str, port: u16) -> Self {
        Self {
            host,
            port,
            tls_option: None,
            support: 0,
        }
    }

    pub fn set_tls_domain(mut self, domain: &'a str) -> Self {
        self.tls_option = Some(domain);
        self
    }

    pub fn support_push(mut self) -> Self {
        self.support |= Support::Push;
        self
    }

    pub fn support_pull(mut self) -> Self {
        self.support |= Support::Pull;
        self
    }

    // 消息流程为 连接后服务器发送服务器信息, 客户端接收后发送客户端信息
    pub async fn connect(mut self) -> Result<Client, Error> {
        let addr = SocketAddr::new(self.host.parse()?, self.port);

        let mut connect = TcpStream::connect(addr).await?;

        let mut buff = [0u8; 1024];
        let mut decode = Decode::new(1024);

        loop {
            let size = connect.read(&mut buff).await?;

            if size == 0 {
                return Err(Error::HandShake(HandShakeError::ConnectClose));
            } else {
                decode.set_buff(&buff[..size]);

                if let Some(message) = decode.iter().next() {
                    if let Message::Info(version, mask, max_message_length) = message? {
                        let mut config = ClientConfig::default();
                        if self.support & Support::Push {
                            config.support_push();
                        }
                        if self.support & Support::Pull {
                            config.support_pull();
                        }
                        connect.write(&config.encode()).await?;

                        let mode = Client::select_mode(&mask, &self.support)?;
                        let stream =
                            Client::select_stream(&mask, &self.tls_option, connect).await?;

                        return Ok(Client {
                            stream,
                            mode,
                            version,
                            max_message_length,
                        });
                    } else {
                        return Err(Error::HandShake(HandShakeError::Parse));
                    }
                } else {
                    continue;
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct Client {
    stream: ConnectType,
    mode: Mode,
    version: u8,
    max_message_length: u32,
}

impl Client {
    fn select_mode(mask: &u16, support: &u16) -> Result<Mode, Error> {
        if !(*support & Support::Push) && !(*support & Support::Pull) {
            return Err(Error::HandShake(HandShakeError::ClientPushOrPull));
        }

        let tmp = *mask & *support;

        if tmp & Support::Push && tmp & Support::Pull {
            return Ok(Mode::PushAndPull);
        } else {
            if tmp & Support::Push {
                return Ok(Mode::Push);
            } else if tmp & Support::Pull {
                return Ok(Mode::Pull);
            } else {
                return Err(Error::HandShake(HandShakeError::ServerPushOrPull));
            }
        }
    }

    async fn select_stream<'a>(
        mask: &u16,
        tls_config: &Option<&'a str>,
        stream: TcpStream,
    ) -> Result<ConnectType, Error> {
        let stream = {
            if let Some(domain) = tls_config {
                if *mask & Support::Tls {
                    let tls_stream = connect(*domain, stream).await?;
                    ConnectType::Tls(tls_stream)
                } else {
                    ConnectType::Normal(stream)
                }
            } else {
                ConnectType::Normal(stream)
            }
        };

        Ok(stream)
    }
}

use super::action::Action;
use super::connect_type::ConnectType;
use super::daemon::Daemon;
use super::error::{Error, HandShakeError};
use super::mode::Mode;
use super::subscription::Subscription;
use async_native_tls::connect;
use protocol::send_to_server::{
    decode::{Decode, Message},
    encode::ClientConfig,
};
use protocol::state::Support;
use smol::channel::{bounded, Sender};
use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::lock::Mutex;
use smol::net::TcpStream;
use smol::spawn;
use std::collections::HashMap;
use std::default::Default;
use std::net::SocketAddr;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc,
};

#[derive(Debug)]
pub struct Builder<'a> {
    host: &'a str,
    port: u16,
    tls_option: Option<&'a str>,
    support: u16,
    max_message_total: Option<usize>,
}

impl<'a> Builder<'a> {
    pub fn new(host: &'a str, port: u16) -> Self {
        Self {
            host,
            port,
            tls_option: None,
            support: 0,
            max_message_total: None,
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

    pub fn set_max_message_total(mut self, total: usize) -> Self {
        self.max_message_total = Some(total);
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
                    if let Message::Info(info) = message? {
                        let mut config = ClientConfig::default();
                        if self.support & Support::Push {
                            config.support_push();
                        }
                        if self.support & Support::Pull {
                            config.support_pull();
                        }
                        connect.write(&config.encode()).await?;

                        let mode = Client::select_mode(&info.support, &self.support)?;
                        let stream =
                            Client::select_stream(&info.support, &self.tls_option, connect).await?;

                        let max_message_length = Arc::from(AtomicU32::new(info.max_message_length));

                        let (sender, receiver) = bounded::<Action>(10);

                        let daemon =
                            Daemon::new(mode, stream, max_message_length.clone(), receiver);
                        spawn(daemon.run(decode)).detach();

                        return Ok(Client {
                            max_message_length,
                            max_task_total: self.max_message_total.unwrap_or(10),
                            daemon_sender: sender,
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
    max_task_total: usize,
    max_message_length: Arc<AtomicU32>,
    daemon_sender: Sender<Action>,
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

    pub async fn subscription(&mut self, sub_name: &str) -> Subscription {
        let (sender, receiver) = bounded(self.max_task_total);

        if let Err(e) = self
            .daemon_sender
            .send(Action::Sub {
                sub_name: sub_name.to_string(),
                msg_sender: sender,
            })
            .await
        {
            panic!("daemon already closed {:?}", e);
        }

        Subscription::new(receiver)
    }

    pub async fn publish<A>(&mut self, sub_name: &str, payload: A)
    where
        A: Into<Vec<u8>>,
    {
        if let Err(e) = self
            .daemon_sender
            .send(Action::Pub {
                sub_name: sub_name.to_string(),
                payload: payload.into(),
            })
            .await
        {
            panic!("daemon already closed {:?}", e);
        }
    }
}

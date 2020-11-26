use super::action::Action;
use super::connect_type::ConnectType;
use super::error::Error;
use super::intval::Intval;
use super::mode::Mode;
use bytes::{Buf, BytesMut};
use futures::future::FutureExt;
use futures::select;
use protocol::send_to_server::{
    decode::{Decode, Message},
    encode::{Err, Ok, Ping, Pong, Pub, Sub, TurnPull, TurnPush, UnSub},
};
use smol::block_on;
use smol::channel::{bounded, Receiver, Sender};
use smol::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;
use std::io::Error as IoError;
use std::ops::Drop;
use std::string::String;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use waitgroup::Worker;
use log::debug;

#[derive(Debug)]
pub(super) struct Daemon {
    mode: Mode,
    stream: ConnectType,
    max_message_length: Arc<AtomicU32>,

    // 定时器
    intval: Intval,

    client_recv: Receiver<(Action, Option<Worker>)>,

    // 订阅, 记录订阅与行为关系
    sub_map: HashMap<String, Sender<BytesMut>>,
}

impl Daemon {
    pub(super) fn new(
        mode: Mode,
        stream: ConnectType,
        max_message_length: Arc<AtomicU32>,
        client_recv: Receiver<(Action, Option<Worker>)>,
    ) -> Self {
        Self {
            mode,
            stream,
            max_message_length,
            intval: Intval::new(30),
            client_recv,
            sub_map: HashMap::new(),
        }
    }

    // 进行维持长连接活动
    pub(super) async fn run(mut self, mut decode: Decode) {
        let mut buff = vec![0; self.max_message_length.load(Ordering::Acquire) as usize];
        'main: loop {
            select! {
               result = FutureExt::fuse(self.stream.read(&mut buff)) => {
                   match result {
                       Ok(0) => {
                          break 'main;
                       },
                       Ok(size) => {
                          self.decode_handle(&mut decode, &buff[..size]).await;
                       },
                       Err(e) => {
                          println!("decode {:?}", e);
                       }
                   }
               },
               result = FutureExt::fuse(self.client_recv.recv()) => {
                  match result {
                      Ok((action, worker)) => {
                          if let Err(e) = self.match_action(action, worker).await {
                            println!("{:?}", e);
                          }
                      }
                      Err(_) => {
                          break;
                      }
                  }
               },
               _ =  FutureExt::fuse(self.intval.run()) => {
                  if let Err(e) = self.send_ping().await {

                  }
               }
            }
        }
    }

    async fn decode_handle(&mut self, decode: &mut Decode, buff: &[u8]) {
        decode.set_buff(&buff);

        for message_result in decode.iter() {
            match message_result {
                Ok(message) => {
                    if let Err(e) = self.match_message(message).await {
                        println!("decode error {:?}", e);
                    }
                }
                Err(e) => {
                    println!("decode error {:?}", e);
                }
            }
        }
    }

    async fn match_message(&mut self, message: Message) -> Result<(), Error> {
        match message {
            Message::Ping => {
                // self.stream.write(Ping::encode()).await?;
            }
            Message::Pong => {
                self.intval.reset().await;
            }
            Message::TurnPush => {
                self.reply_turn_push().await?;
            }
            Message::TurnPull => {
                self.reply_turn_pull().await?;
            }
            Message::Msg(msg) => {
                debug!("msg {:?}", msg);
                let sub_name = String::from_utf8(msg.sub_name.to_vec())?;
                let payload = msg.payload;
                self.recv_msg(sub_name, payload).await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn send_ping(&mut self) -> Result<(), IoError> {
        self.stream.write(Ping::encode()).await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn reply_turn_push(&mut self) -> Result<(), IoError> {
        if self.mode.can_push() {
            self.stream.write(Ok::encode()).await?;
        } else {
            self.stream
                .write(&Err::new("Client not support push").encode())
                .await?;
        }
        self.stream.flush().await?;
        Ok(())
    }

    async fn reply_turn_pull(&mut self) -> Result<(), IoError> {
        if self.mode.can_pull() {
            self.stream.write(Ok::encode()).await?;
        } else {
            self.stream
                .write(&Err::new("Client not support pull").encode())
                .await?;
        }
        self.stream.flush().await?;
        Ok(())
    }

    async fn send_sub(&mut self, sub_name: &str) -> Result<(), IoError> {
        self.stream.write(&Sub::new(sub_name).encode()[..]).await?;
        self.stream.flush().await?;
        debug!("send_sub finish");
        Ok(())
    }

    async fn send_pub<A>(&mut self, sub_name: &str, payload: A) -> Result<(), IoError>
    where
        A: AsRef<[u8]>,
    {
        self.stream
            .write(&Pub::new(sub_name, payload).encode())
            .await?;
        self.stream.flush().await?;
        Ok(())
    }

    async fn send_unsub(&mut self, unsub_payload: BytesMut) -> Result<(), IoError> {
        self.stream.write(unsub_payload.bytes()).await?;
        self.stream.flush().await?;
        Ok(())
    }

    // 从服务器那边接受消息
    async fn recv_msg<'a>(&mut self, sub_name: String, msg: BytesMut) {
        if let Some(sender) = self.sub_map.get_mut(&sub_name) {
            if let Err(_) = sender.send(msg).await {
                self.sub_map.remove(&sub_name);
            }
        }
    }

    async fn match_action(
        &mut self,
        action: Action,
        wait_group: Option<Worker>,
    ) -> Result<(), Error> {
        match action {
            Action::Sub {
                sub_name,
                msg_sender,
            } => {
                self.set_sub(sub_name, msg_sender).await?;
                if let Some(worker) = wait_group {
                    drop(worker);
                    debug!("drop worker finish");
                }
            }
            Action::Pub { sub_name, payload } => {
                self.set_publish(sub_name, payload).await?;
                if let Some(worker) = wait_group {
                    drop(worker);
                }
            }
        }

        Ok(())
    }

    async fn set_sub(
        &mut self,
        sub_name: String,
        subscription_sender: Sender<BytesMut>,
    ) -> Result<(), IoError> {
        self.send_sub(&sub_name).await?;
        self.sub_map.insert(sub_name, subscription_sender);
        Ok(())
    }

    async fn set_publish(&mut self, sub_name: String, payload: Vec<u8>) -> Result<(), IoError> {
        self.send_pub(&sub_name, payload).await
    }
}

impl Drop for Daemon {
    //  折构时尝试发送取消订阅到服务端
    fn drop(&mut self) {
        block_on(async {
            let mut unsub = UnSub::new();
            self.sub_map.keys().for_each(|sub_name| {
                unsub.push(sub_name.as_bytes());
            });

            self.send_unsub(unsub.encode()).await;
        });
    }
}

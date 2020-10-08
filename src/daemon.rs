use super::connect_type::ConnectType;
use super::error::Error;
use super::intval::Intval;
use super::mode::Mode;
use futures::future::FutureExt;
use futures::select;
use protocol::send_to_server::{
    decode::{Decode, Message},
    encode::{Ok, Ping, Pong, TurnPull, TurnPush},
};
use smol::channel::{bounded, Receiver, Sender};
use smol::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

#[derive(Debug)]
pub(super) struct Daemon {
    mode: Mode,
    stream: ConnectType,
    max_message_length: Arc<AtomicU32>,

    // 定时器
    intval: Intval,
}

impl Daemon {
    pub(super) fn new(mode: Mode, stream: ConnectType, max_message_length: Arc<AtomicU32>) -> Self {
        Self {
            mode,
            stream,
            max_message_length,
            intval: Intval::new(30),
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
                          decode.set_buff(&buff[..size]);

                          for message_result in decode.iter() {
                             match message_result {
                                 Ok(message) => {
                                     if let Err(e) = self.match_message(message).await {

                                     }
                                 }
                                 Err(e) => {

                                 }
                             }
                          }
                       },
                       Err(e) => {

                       }
                   }
               },
               _ =  FutureExt::fuse(self.intval.run()) => {
                    self.stream.write(Ping::encode()).await;
                    self.stream.flush().await;
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
                self.intval.reset();
            }
            Message::TurnPush => {
                if self.mode.can_push() {
                    self.stream.write(Ok::encode()).await?;
                } else {
                }
            }
            Message::TurnPull => {
                if self.mode.can_pull() {
                    self.stream.write(Ok::encode()).await?;
                } else {
                }
            }
            _ => {}
        }
        Ok(())
    }
}

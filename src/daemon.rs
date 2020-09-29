use super::connect_type::ConnectType;
use super::mode::Mode;
use smol::io::{AsyncReadExt, AsyncWriteExt};
use smol::channel::{Receiver, Sender, bounded};
use smol::Timer;
use super::error::Error;
use futures::select;
use protocol::send_to_server::{
  decode::{Decode, Message},
  encode::{Ping, Pong},
};
use std::sync::Arc;
use std::sync::atomic::{
  AtomicU32,
  Ordering,
};
use std::time::Instant;

#[derive(Debug)]
pub(super) struct Daemon {
    mode: Mode,
    stream: ConnectType,
    max_message_size: Arc<AtomicU32>,
}

impl Daemon {
    pub(super) fn new(mode: Mode, stream: ConnectType, max_message_size: Arc<AtomicU32>) -> Self {
        Self {
            mode,
            stream,
            max_message_size,
        }
    }

    pub(super) async fn run(mut self, decode: Decode) {
        
        let mut buff = vec![0; self.max_message_size.load(Ordering::Acquire) as usize];
        'main: loop {
           select! {
              result = self.stream.read(&mut buff) => {
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
              }
           } 
        }
    }

    async fn match_message(&mut self, message: Message) -> Result<(), Error> {
        match message {
            Message::Ping => {
                self.stream.write(Ping::encode()).await;
            },
            Message::Pong => {
                self.stream.write(Pong::encode()).await;
            },
            Message::TurnPush => {
                if self.mode.can_push() {
                
                } else {
                
                }
            },
            Message::TurnPull => {
                if self.mode.can_pull() {
                
                } else {
                
                }
            },
            _ => {
            
            }
        }
        Ok(())
    }
}

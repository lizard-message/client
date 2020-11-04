use smol::channel::Receiver;
use smol::spawn;
use std::marker::Send;
use std::ops::FnMut;

pub struct Subscription {
    recv: Receiver<String>,
}

impl Subscription {
    pub(super) fn new(recv: Receiver<String>) -> Self {
        Self { recv }
    }

    pub async fn with_handle<F>(self, mut proccess: F)
    where
        F: FnMut(String) + Send + 'static,
    {
        loop {
            match self.recv.recv().await {
                Ok(msg) => proccess(msg),
                Err(e) => {
                    break;
                }
            }
        }
    }
}

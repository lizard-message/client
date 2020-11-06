use bytes::BytesMut;
use smol::channel::Receiver;
use smol::stream::Stream;
use std::borrow::Cow;
use std::marker::Send;
use std::ops::FnMut;
use std::pin::Pin;
use std::string::{FromUtf8Error, String};
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct Subscription {
    recv: Receiver<BytesMut>,
}

impl Subscription {
    pub(super) fn new(recv: Receiver<BytesMut>) -> Self {
        Self { recv }
    }

    pub async fn with_bytes_handle<F>(self, mut proccess: F)
    where
        F: FnMut(BytesMut) + Send + 'static,
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

    pub async fn with_string_handle<F>(self, mut proccess: F)
    where
        F: FnMut(Cow<'_, str>) + Send + 'static,
    {
        loop {
            match self.recv.recv().await {
                Ok(msg) => {
                    let msg_string = String::from_utf8_lossy(&msg);
                    proccess(msg_string);
                }
                Err(e) => {
                    break;
                }
            }
        }
    }

    pub fn get_bytes_stream(&mut self) -> BytesIter<'_> {
        BytesIter { iter: self }
    }

    pub fn get_string_stream(&mut self) -> StrIter<'_> {
        StrIter { iter: self }
    }
}

#[derive(Debug)]
pub struct BytesIter<'a> {
    iter: &'a mut Subscription,
}

impl<'a> Stream for BytesIter<'a> {
    type Item = BytesMut;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let my = self.get_mut();

        Stream::poll_next(Pin::new(&mut (my.iter.recv)), cx)
    }
}

#[derive(Debug)]
pub struct StrIter<'a> {
    iter: &'a mut Subscription,
}

impl<'a> Stream for StrIter<'a> {
    type Item = Result<String, FromUtf8Error>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let my = self.get_mut();

        Stream::poll_next(Pin::new(&mut (my.iter.recv)), cx)
            .map(|map| map.map(|item| String::from_utf8(item.to_vec())))
    }
}

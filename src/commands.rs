use futures::prelude::*;
use futures::stream::Stream;

use redis::r#async::SharedConnection;

use telebot::objects::Message;
use telebot::bot::RequestHandle;

use failure::Error;

use std::boxed::Box;

pub struct DatabaseCommand {
    stream: Box<Stream<Item = (RequestHandle, Message), Error = Error> + Send>,
    conn: SharedConnection,
}

impl DatabaseCommand {
    pub fn new(
        command: impl Stream<Item = (RequestHandle, Message), Error = Error> + Send + 'static,
        conn: SharedConnection,
    ) -> Self {
        Self {
            stream: Box::new(command),
            conn: conn,
        }
    }
}

impl Stream for DatabaseCommand {
    type Item = (RequestHandle, Message, SharedConnection);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Error> {
        match self.stream.poll() {
            Ok(result) => match result {
                Async::Ready(item) => match item {
                    Some(value) => Ok(Async::Ready(Some((value.0, value.1, self.conn.clone())))),
                    None => Ok(Async::Ready(None)),
                },
                Async::NotReady => Ok(Async::NotReady),
            },
            Err(err) => Err(err),
        }
    }
}

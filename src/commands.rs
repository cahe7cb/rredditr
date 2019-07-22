use futures::prelude::*;
use futures::stream::Stream;

use redis::r#async::SharedConnection;

use telebot::bot::RequestHandle;
use telebot::objects::Message;

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

pub struct BotCommands {
    conn: SharedConnection,
    streams: Vec<Box<Stream<Item = (RequestHandle, Message), Error = Error> + Send>>,
}

impl BotCommands {
    pub fn new(conn: SharedConnection) -> Self {
        Self {
            conn: conn,
            streams: Vec::<Box<Stream<Item = (RequestHandle, Message), Error = Error> + Send>>::new(
            ),
        }
    }

    pub fn create(
        &self,
        command: impl Stream<Item = (RequestHandle, Message), Error = Error> + Send + 'static,
    ) -> impl Stream<Item = (RequestHandle, Message, SharedConnection), Error = Error> {
        DatabaseCommand::new(command, self.conn.clone())
    }

    pub fn add_command(
        &mut self,
        command: impl Stream<Item = (RequestHandle, Message), Error = Error> + Send + 'static,
    ) {
        self.streams.push(Box::new(command));
    }
}

impl Stream for BotCommands {
    type Item = (RequestHandle, Message);
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Error> {
        let mut index = 0;
        while index < self.streams.len() {
            let stream = self.streams.get_mut(index).unwrap();
            let poll = stream.poll();
            if let Ok(result) = poll {
                if let Async::Ready(item) = result {
                    if let Some(value) = item {
                        return Ok(Async::Ready(Some(value)));
                    } else {
                        self.streams.remove(index);
                    }
                } else {
                    index = index + 1;
                }
            } else if let Err(err) = poll {
                self.streams.remove(index);
                return Err(err);
            }
        }
        Ok(Async::NotReady)
    }
}

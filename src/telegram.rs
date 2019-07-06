use telebot::bot::RequestHandle;
use telebot::functions::FunctionSendMessage;
use telebot::objects::Message;

use futures::prelude::*;

use redis::r#async::SharedConnection;

use crate::commands::DatabaseCommand;

fn handle_available(
    handle: RequestHandle,
    message: Message,
    conn: SharedConnection,
) -> impl Future<Item = (), Error = ()> {
    redis::cmd("SMEMBERS")
        .arg("main/subs")
        .query_async::<_, Vec<String>>(conn)
        .map_err(|err: redis::RedisError| {
            eprintln!("Failed to retrieve available subreddits: {:#?}", err);
        })
        .and_then(move |(_conn, results)| {
            let mut reply = String::from("These are the available subreddits:\n");
            for sub in results {
                reply.push_str(format!("- {}\n", sub).as_str());
            }
            handle.message(message.chat.id, reply).send()
                .map_err(|_| {})
        })
        .map(|_| {})
}

fn handle_unsubscribe(
    handle: RequestHandle,
    message: Message,
    conn: SharedConnection,
) -> impl Future<Item = (), Error = ()> {
    let target = match message.text.clone() {
        Some(str) => str,
        None => String::new(),
    };
    redis::cmd("SREM")
        .arg(format!("subscribers/{}", target))
        .arg(message.chat.id)
        .query_async::<_, ()>(conn)
        .map_err(|err: redis::RedisError| {
            eprintln!("Failed to unsubscribe from subreddit: {:#?}", err);
        })
        .and_then(move |_| {
            handle
                .message(
                    message.chat.id,
                    format!("You are no longer subscribed to {}", target),
                )
                .send()
                .map_err(|_| {})
        })
        .map(|_| {})
}
fn handle_subscribe(
    handle: RequestHandle,
    message: Message,
    conn: SharedConnection,
) -> impl Future<Item = (), Error = ()> {
    let target = match message.text.clone() {
        Some(str) => str,
        None => String::new(),
    };
    redis::cmd("SISMEMBER")
        .arg(format!("main/subs"))
        .arg(&target)
        .query_async::<_, bool>(conn)
        .map_err(|err: redis::RedisError| {
            eprintln!("Failed checking for existence of subreddit: {:#?}", err);
        })
        .and_then(|(conn, _exists)| {
            redis::cmd("SADD")
                .arg(format!("subscribers/{}", target))
                .arg(message.chat.id)
                .query_async::<_, ()>(conn)
                .map_err(|err: redis::RedisError| {
                    eprintln!("Failed to subscribe to: {:#?}", err);
                })
                .and_then(move |_| {
                    handle
                        .message(
                            message.chat.id,
                            format!("You are now subscribed to {}", target),
                        )
                        .send()
                        .map_err(|_| {})
                })
        })
        .map(|_| {})
        .map_err(|_| {})
}

pub fn telegram_context(conn: SharedConnection, mut bot: telebot::Bot) {
    tokio::executor::spawn(
        DatabaseCommand::new(bot.new_cmd("/sub"), conn.clone())
            .map_err(|_| {})
            .for_each(|(handle, msg, conn)| handle_subscribe(handle, msg, conn)),
    );
    tokio::executor::spawn(
        DatabaseCommand::new(bot.new_cmd("/unsub"), conn.clone())
            .map_err(|_| {})
            .for_each(|(handle, msg, conn)| handle_unsubscribe(handle, msg, conn)),
    );
    tokio::executor::spawn(
        DatabaseCommand::new(bot.new_cmd("/subs"), conn.clone())
            .map_err(|_| {})
            .for_each(|(handle, msg, conn)| handle_available(handle, msg, conn)),
    );
    tokio::executor::spawn(bot.into_future().map_err(|err| {
        eprintln!("Telegram API error: {:#?}", err);
    }));
}

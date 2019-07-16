use telebot::bot::RequestHandle;
use telebot::functions::FunctionSendMessage;
use telebot::objects::Message;

use futures::prelude::*;

use redis::r#async::SharedConnection;

use crate::commands::DatabaseCommand;
use crate::database::Database;

fn handle_available(
    handle: RequestHandle,
    message: Message,
    conn: SharedConnection,
) -> impl Future<Item = (), Error = ()> {
    Database::fetch_available_subs(conn)
        .and_then(move |(_conn, results)| {
            let mut reply = String::from("These are the available subreddits:\n");
            for sub in results {
                reply.push_str(format!("- {}\n", sub).as_str());
            }
            handle
                .message(message.chat.id, reply)
                .send()
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
    Database::delete_subscriber(conn, &target, message.chat.id)
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
    Database::fetch_is_sub_available(conn, &target)
        .then(move |result| {
            let chat = message.chat.id;
            if let Ok((conn, exists)) = result {
                if exists {
                    Ok((conn, handle, chat, target))
                } else {
                    Err((handle, chat, target))
                }
            } else {
                Err((handle, chat, target))
            }
        })
        .map_err(|(handle, chat, target)| {
            tokio::executor::spawn(
                handle
                    .message(chat, format!("The subreddit {} is not available", target))
                    .send()
                    .map(|_| {})
                    .map_err(|_| {}),
            );
        })
        .and_then(|(conn, handle, chat, target)| {
            Database::insert_subscriber(conn, &target, chat)
                .and_then(move |_| {
                    handle
                        .message(chat, format!("You are now subscribed to {}", target))
                        .send()
                        .map_err(|_| {})
                })
        })
        .map(|_| {})
        .or_else(|_| Ok(()))
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

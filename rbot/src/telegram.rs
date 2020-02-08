use telebot::bot::RequestHandle;
use telebot::functions::FunctionSendMessage;
use telebot::objects::Message;

use futures::prelude::*;

use redis::aio::SharedConnection;

use crate::commands::BotCommands;
use crate::database::Database;

use failure::Error;

use std::boxed::Box;

fn handle_available(
    handle: RequestHandle,
    message: Message,
    conn: SharedConnection,
) -> impl Future<Item = (RequestHandle, Message), Error = Error> {
    Database::fetch_available_subs(conn)
        .from_err()
        .and_then(move |(_conn, results)| {
            let mut reply = String::from("These are the available subreddits:\n");
            for sub in results {
                reply.push_str(format!("- {}\n", sub).as_str());
            }
            handle.message(message.chat.id, reply).send()
        })
}

fn handle_unsubscribe(
    handle: RequestHandle,
    message: Message,
    conn: SharedConnection,
) -> impl Future<Item = (RequestHandle, Message), Error = Error> {
    let target = match message.text.clone() {
        Some(str) => str,
        None => String::new(),
    };
    Database::delete_subscriber(conn, &target, message.chat.id)
        .from_err()
        .and_then(move |_| {
            handle
                .message(
                    message.chat.id,
                    format!("You are no longer subscribed to {}", target),
                )
                .send()
        })
}

fn subscribe_insert_subscriber(
    handle: RequestHandle,
    chat: i64,
    target: String,
    conn: SharedConnection,
) -> Box<dyn Future<Item = (RequestHandle, Message), Error = Error> + Send> {
    Box::new(
        Database::insert_subscriber(conn, &target, chat)
            .from_err()
            .and_then(move |_| {
                handle
                    .message(chat, format!("You are now subscribed to {}", target))
                    .send()
            }),
    )
}

fn subscribe_subreddit_not_available(
    handle: RequestHandle,
    chat: i64,
    target: String,
) -> Box<dyn Future<Item = (RequestHandle, Message), Error = Error> + Send> {
    Box::new(
        handle
            .message(chat, format!("The subreddit {} is not available", target))
            .send(),
    )
}

fn handle_subscribe(
    handle: RequestHandle,
    message: Message,
    conn: SharedConnection,
) -> impl Future<Item = (RequestHandle, Message), Error = Error> + Send {
    let target = match message.text.clone() {
        Some(str) => str,
        None => String::new(),
    };
    Database::fetch_is_sub_available(conn, &target)
        .from_err()
        .and_then(move |(conn, exists)| {
            if exists {
                subscribe_insert_subscriber(handle, message.chat.id, target, conn)
            } else {
                subscribe_subreddit_not_available(handle, message.chat.id, target)
            }
        })
}

pub fn build_commands(bot: &mut telebot::Bot, conn: SharedConnection) -> BotCommands {
    let mut commands = BotCommands::new(conn);
    commands.add_command(
        commands
            .create(bot.new_cmd("/sub"))
            .and_then(|(handle, msg, conn)| handle_subscribe(handle, msg, conn)),
    );
    commands.add_command(
        commands
            .create(bot.new_cmd("/unsub"))
            .and_then(|(handle, msg, conn)| handle_unsubscribe(handle, msg, conn)),
    );
    commands.add_command(
        commands
            .create(bot.new_cmd("/subs"))
            .and_then(|(handle, msg, conn)| handle_available(handle, msg, conn)),
    );
    commands.add_command(
        commands
            .create(bot.unknown_cmd())
            .and_then(|(handle, msg, _conn)| {
                handle
                    .message(msg.chat.id, format!("I don't understand that"))
                    .send()
            }),
    );
    commands
}

pub fn telegram_context(conn: SharedConnection, bot: &mut telebot::Bot) {
    let commands = build_commands(bot, conn);
    tokio::executor::spawn(
        commands
            .map_err(|err| {
                eprintln!("Error handling command: {:#?}", err);
            })
            .for_each(|_| Ok(())),
    );
}

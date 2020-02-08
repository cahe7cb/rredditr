use std::env;

use futures::*;

mod telegram;
mod commands;
mod database;

use telebot::*;
use telebot::functions::FunctionSendMessage;

use redis::Commands;

use serde::Deserialize;

#[derive(Deserialize, Clone)]
pub struct Message {
    pub target: i64,
    pub link: String,
}

fn main() {
    let addr = env::var("REDIS_ADDRESS").expect("No database address specified");
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("Telegram API token not found");
    let mut bot = Bot::new(token.as_str()).update_interval(1000);

    let redis = redis::Client::open(addr.as_str()).expect("Could not find database");
    let mut conn = redis.get_connection().expect("Failed to estabilishing worker database connection");
    let handle = bot.request.clone();

    std::thread::spawn(move || {
        let mut rt = tokio::runtime::Runtime::new().expect("Failed to start sender runtime");
        loop {
            if let Ok(payload) = conn.brpoplpush::<_, String>("main/messages", "workers/1", 1) {
                let message = serde_json::from_str::<Message>(payload.as_str()).unwrap();
                let send_message = handle.message(message.target, message.link).send();
                if let Err(err) = rt.block_on(send_message) {
                    eprintln!("Failed to send link to {}, will retry: {:#?}", message.target, err);
                    conn.rpoplpush::<_, ()>("workers/1", "main/messages").expect("Failed queueing failed message");
                }
                else {
                    conn.rpop::<_, ()>("workers/1").expect("Failed to dequeue sent message");
                }
            }
        }
    });

    tokio::run(
        redis
            .get_shared_async_connection()
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed to establish bot database connection: {:#?}", err);
            })
            .and_then(move |conn| {
                telegram::telegram_context(conn, &mut bot);
                bot.into_future()
                    .map_err(|err| {
                        eprintln!("Telegram API error: {:#?}", err);
                    })
            }),
    );
}

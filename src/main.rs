#![recursion_limit = "128"]

use std::env;

use futures::*;

mod reddit;
mod telegram;
mod worker;
mod commands;
mod database;

use telebot::*;

use database::Database;

type SharedDependency = (
    reqwest::r#async::Client,
    redis::r#async::SharedConnection,
    bot::RequestHandle,
);

fn new_update_delay() -> impl future::Future<Item = (), Error = ()> {
    tokio::timer::Delay::new(std::time::Instant::now() + std::time::Duration::from_secs(30))
        .map_err(|err| {
            eprintln!("Worker timer error: {:#?}", err);
        })
}

fn update_available_subs(
    iter: impl std::iter::Iterator<Item = String>,
    shared: SharedDependency,
) -> impl Future<Item = SharedDependency, Error = ()> {
    future::loop_fn((iter, shared), |(mut iter, shared)| {
        new_update_delay().and_then(|_| {
            if let Some(next) = iter.next() {
                tokio::executor::spawn(worker::handle_update(
                    &shared.0,
                    shared.1.clone(),
                    shared.2.clone(),
                    next,
                ));
                Ok(future::Loop::Continue((iter, shared)))
            } else {
                Ok(future::Loop::Break(shared))
            }
        })
    })
}

fn updater_worker_context(
    http: reqwest::r#async::Client,
    conn: redis::r#async::SharedConnection,
    bot: telebot::bot::RequestHandle,
) -> impl Future<Item = (), Error = ()> {
    future::loop_fn((http, conn, bot), |(http, conn, bot)| {
        Database::fetch_available_subs(conn)
            .and_then(|(conn, subs)| update_available_subs(subs.into_iter(), (http, conn, bot)))
            .and_then(|(http, conn, bot)| Ok(future::Loop::Continue((http, conn, bot))))
    })
}

fn main() {
    let addr = env::var("REDIS_ADDRESS").expect("No database address specified");
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("Telegram API token not found");
    let bot = Bot::new(token.as_str()).update_interval(1000);

    let redis = redis::Client::open(addr.as_str()).expect("Could not find database");
    let http = reqwest::r#async::Client::new();

    tokio::run(
        redis
            .get_shared_async_connection()
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed to establish worker database connection: {:#?}", err);
            })
            .and_then(move |conn| {
                telegram::telegram_context(conn.clone(), bot.clone());
                updater_worker_context(http, conn.clone(), bot.request)
            }),
    );
}

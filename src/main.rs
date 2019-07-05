#![recursion_limit = "128"]

use std::env;

use futures::*;

mod reddit;
mod telegram;
mod worker;

type SharedDependency = (
    reqwest::r#async::Client,
    redis::r#async::SharedConnection,
    tokio::sync::mpsc::Sender<(i64, Vec<String>)>,
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
    send: tokio::sync::mpsc::Sender<(i64, Vec<String>)>,
) -> impl Future<Item = (), Error = ()> {
    future::loop_fn((http, conn, send), |(http, conn, send)| {
        redis::cmd("SMEMBERS")
            .arg("main/subs")
            .query_async::<_, Vec<String>>(conn)
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed to update subreddits list: {:#?}", err);
            })
            .and_then(|(conn, subs)| update_available_subs(subs.into_iter(), (http, conn, send)))
            .and_then(|(http, conn, send)| Ok(future::Loop::Continue((http, conn, send))))
    })
}

fn main() {
    let addr = env::var("REDIS_ADDRESS").expect("No database address specified");
    let token = env::var("TELEGRAM_BOT_TOKEN").expect("Telegram API token not found");

    let redis = redis::Client::open(addr.as_str()).expect("Could not find database");
    let http = reqwest::r#async::Client::new();

    let (send, updates) = tokio::sync::mpsc::channel::<(i64, Vec<String>)>(1024 * 1024);

    telegram::start_worker(&redis, updates, token);

    tokio::run(
        redis
            .get_shared_async_connection()
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed to establish worker database connection: {:#?}", err);
            })
            .and_then(|conn| updater_worker_context(http, conn, send)),
    );
}

use std::env;

mod database;
mod reddit;
mod worker;

async fn run_bot(
    redis : redis::Client,
    http : reqwest::Client
) -> Result<(), ()> {
    let (conn, redis_poll) = redis.get_multiplexed_async_connection().await
        .map_err(|err| log::error!("Failed to estabilish worker database connection"))?;
    tokio::spawn(
        crate::worker::updater_worker_context(http, conn)
    );
    Ok(redis_poll.await)
}

fn main() {
    let addr = env::var("REDIS_ADDRESS").expect("No database address specified");

    let redis = redis::Client::open(addr).expect("Could not find database");
    let http = reqwest::Client::builder().user_agent("Rredditr bot").build().unwrap();

    env_logger::init();

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(run_bot(redis, http)).unwrap();
}

use tokio_core::reactor::Core;

use redis::Commands;

use tokio::sync::mpsc;

use futures::*;
use telegram_bot::*;

fn telegram_worker_context(
    redis: redis::Connection,
    updates: mpsc::Receiver<(i64, Vec<String>)>,
    token: String,
) {
    let mut core = Core::new().unwrap();
    let telegram = Api::configure(token)
        .build(core.handle())
        .expect("Failed to configure Telegram API");

    let commands = telegram
        .stream()
        .for_each(|update| {
            if let UpdateKind::Message(message) = update.kind {
                if let MessageKind::Text { ref data, .. } = message.kind {
                    if data.starts_with("/subs") {
                        let subs: Vec<String> = redis.smembers("main/subs").unwrap();
                        let mut response = String::from("Available subs\n");
                        for sub in &subs {
                            response.push_str(&format!("{}\n", sub));
                        }
                        telegram.spawn(message.text_reply(response));
                    } else if data.starts_with("/sub") {
                        let arg: Vec<&str> = data.split(' ').collect();
                        if let Some(sub) = arg.get(1) {
                            if redis
                                .sismember::<&str, &str, bool>("main/subs", *sub)
                                .unwrap()
                            {
                                let _: () = redis
                                    .sadd(
                                        format!("subscribers/{}", sub),
                                        i64::from(message.chat.id()),
                                    )
                                    .unwrap();
                            }
                        }
                    } else if data.starts_with("/unsub") {
                        let v: Vec<&str> = data.split(' ').collect();
                        if let Some(sub) = v.get(1) {
                            let _: () = redis
                                .srem(format!("subscribers/{}", sub), i64::from(message.chat.id()))
                                .unwrap();
                        }
                    }
                }
            }
            Ok(())
        })
        .map_err(|err| {
            eprintln!("Telegram stream error: {:#?}", err);
        });

    let dispatcher = updates.map_err(|_| ()).for_each(|(subscriber, batch)| {
        let chat = ChatId::new(subscriber);
        future::loop_fn((chat, batch.into_iter()), |(chat, mut iter)| {
            let mut send = vec![];
            if let Some(message) = iter.next() {
                send.push(
                    telegram
                        .send(chat.text(message))
                        .and_then(|_response| Ok(()))
                        .map_err(|err| {
                            eprintln!("Telegram request error: {:#?}", err);
                        }),
                )
            }
            future::join_all(send).and_then(move |result| match result.len() {
                0 => Ok(future::Loop::Break(())),
                _ => Ok(future::Loop::Continue((chat, iter))),
            })
        })
    });

    core.run(commands.join(dispatcher))
        .expect("The bot is dead");
}

pub fn start_worker(
    client: &redis::Client,
    updates: mpsc::Receiver<(i64, Vec<String>)>,
    token: String,
) {
    let redis = client
        .get_connection()
        .expect("Failed to connect to database");

    std::thread::spawn(|| {
        telegram_worker_context(redis, updates, token);
    });
}

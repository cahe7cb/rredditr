use crate::reddit::decode_submission_array;
use crate::reddit::Submission;
use crate::reddit::RECENT_OFFSET;

use futures::*;

use serde_json::Value;

use crate::database::Database;

use log::error;

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Message {
    pub target: i64,
    pub link: String,
}

async fn get_subscribers(
    conn: &mut redis::aio::MultiplexedConnection,
    subreddit: &String,
) ->  Result<Vec<i64>, ()> {
    Database::fetch_subscribers(conn, subreddit)
        .map_err(|err: redis::RedisError| {
            error!("Failed to load subscribers for {}: {:#?}", subreddit, err);
        })
        .await
}

async fn filter_new_submissions(
    conn : &mut redis::aio::MultiplexedConnection,
    submissions: Vec<Submission>,
) -> Result<Vec<Submission>, ()> {
    let now = chrono::Utc::now().timestamp() as f64;
    let keys: Vec<String> = submissions.iter().map(|post| post.get_key()).collect();
    let statuses = Database::fetch_submissions_status(conn, keys)
        .map_err(|err : redis::RedisError| error!("Failed to fetch submissions status: {:#?}", err))
        .await?;
    let news = submissions
        .into_iter()
        .enumerate()
        .filter_map(|(i, p)| match statuses.get(i) {
            Some(t) => match t {
                redis::Value::Nil => Some(p),
                _ => None,
            },
            None => None,
        })
        .filter(|p| p.created_utc > (now - RECENT_OFFSET))
        .collect();
    Ok(news)
}

pub async fn fetch_submissions(http : &reqwest::Client, subreddit : &String) -> Result<Vec<Submission>, ()> {
    let url = format!("https://api.reddit.com/r/{}/new.json", subreddit);
    let response = http.get(url.as_str())
        .send()
        .map_err(|err: reqwest::Error| {
            log::error!("Request error: {:#?}", err);
        })
        .await?;
    let data = response
        .json::<Value>()
        .map_err(|err: reqwest::Error| {
            log::error!("Failed to parse response: {:#?}", err);
        })
        .await?;
    Ok(decode_submission_array(data))
}

pub async fn handle_update(
    http: &reqwest::Client,
    conn: &mut redis::aio::MultiplexedConnection,
    subreddit: String,
) -> Result<(), ()> {
    let submissions = fetch_submissions(&http, &subreddit).await?;
    let subscribers = get_subscribers(conn, &subreddit).await?;
    let news = filter_new_submissions(conn, submissions).await?;
    for s in subscribers.iter() {
        for n in news.iter() {
            let message = Message{
                target: *s,
                link: n.get_url()
            };
            Database::queue_message(conn, serde_json::to_string(&message).unwrap())
                .map_err(|err| log::error!("Failed sending messages batch from {}: {:#?}", subreddit, err))
                .await?;
        }
    }
    Database::update_submissions(conn, news.clone())
        .map_err(|err| log::error!("Failed updating new submissions: {:#?}", err))
        .await
}

pub async fn updater_worker_context(
    http: reqwest::Client,
    mut conn: redis::aio::MultiplexedConnection,
) -> Result<(), ()> {
    loop {
        let subs = Database::fetch_available_subs(&mut conn)
            .await
            .map_err(|err : redis::RedisError| error!("Database error fetching available subs: {:#?}", err))?;
        for next in subs {
            log::info!("Updating {}", next);
            handle_update(&http, &mut conn, next).await?;
            tokio::time::delay_for(std::time::Duration::from_secs(30)).await;
        }
    }
}

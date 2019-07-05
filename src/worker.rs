use crate::reddit::decode_submission_array;
use crate::reddit::Submission;
use crate::reddit::RECENT_OFFSET;

use futures::*;
use tokio::sync::mpsc::*;

use serde_json::Value;

fn dispatch_news(
    mut send: Sender<(i64, Vec<String>)>,
    news: Vec<Submission>,
    subscribers: Vec<i64>,
) {
    let batch: Vec<String> = news.into_iter().map(|post| post.get_url()).collect();
    for subscriber in subscribers {
        send.try_send((subscriber, batch.clone()))
            .expect("Failed to send update batch");
    }
}

fn get_subscribers(
    conn: redis::r#async::SharedConnection,
    subreddit: String,
    news: Vec<Submission>,
) -> impl Future<Item = (Vec<i64>, Vec<Submission>), Error = ()> {
    redis::cmd("SMEMBERS")
        .arg(format!("subscribers/{}", subreddit))
        .query_async::<_, Vec<i64>>(conn)
        .map(|(_, subscribers)| (subscribers, news))
        .map_err(move |err: redis::RedisError| {
            eprintln!("Failed to load subscribers for {}: {:#?}", subreddit, err);
        })
}

fn update_new_submissions(
    conn: redis::r#async::SharedConnection,
    news: Vec<Submission>,
) -> impl Future<Item = (redis::r#async::SharedConnection, Vec<Submission>), Error = ()> {
    let now = chrono::Utc::now().timestamp() as f64;
    let pipe = news.iter().fold(redis::pipe(), |mut pipe, post| {
        pipe.cmd("SETEX")
            .arg((post.get_key(), 2.0 * RECENT_OFFSET, post.get_tag(now)))
            .to_owned()
    });
    pipe.query_async::<_, ()>(conn)
        .and_then(|(conn, _)| Ok((conn, news)))
        .map_err(|err: redis::RedisError| {
            eprintln!("Failed to update posts: {:#?}", err);
        })
}

fn filter_new_submissions(
    values: Vec<redis::Value>,
    submissions: Vec<Submission>,
) -> Vec<Submission> {
    let now = chrono::Utc::now().timestamp() as f64;

    submissions
        .into_iter()
        .enumerate()
        .filter_map(|(i, p)| match values.get(i) {
            Some(t) => match t {
                redis::Value::Nil => Some(p),
                _ => None,
            },
            None => None,
        })
        .filter(|p| p.created_utc > (now - RECENT_OFFSET))
        .collect()
}

fn process_submissions(
    conn: redis::r#async::SharedConnection,
    submissions: Vec<Submission>,
) -> impl Future<Item = (redis::r#async::SharedConnection, Vec<Submission>), Error = ()> {
    let keys: Vec<String> = submissions.iter().map(|post| post.get_key()).collect();

    redis::cmd("MGET")
        .arg(keys)
        .query_async::<_, Vec<redis::Value>>(conn)
        .map_err(|err: redis::RedisError| {
            eprintln!("Failed retrieving posts status: {:#?}", err);
        })
        .and_then(|(conn, values)| {
            update_new_submissions(conn, filter_new_submissions(values, submissions))
        })
}

fn parse_response(
    mut response: reqwest::r#async::Response,
) -> impl Future<Item = Value, Error = ()> {
    response.json().map_err(|err: reqwest::Error| {
        eprintln!("Failed to parse response: {:#?}", err);
    })
}

fn get_subreddit_data(
    http: &reqwest::r#async::Client,
    subreddit: &String,
) -> impl Future<Item = reqwest::r#async::Response, Error = ()> {
    let url = format!("https://api.reddit.com/r/{}/new.json", subreddit);
    http.get(url.as_str())
        .send()
        .map_err(|err: reqwest::Error| {
            eprintln!("Request error: {:#?}", err);
        })
}

pub fn handle_update(
    http: &reqwest::r#async::Client,
    conn: redis::r#async::SharedConnection,
    send: Sender<(i64, Vec<String>)>,
    subreddit: String,
) -> impl Future<Item = (), Error = ()> {
    get_subreddit_data(&http, &subreddit)
        .and_then(parse_response)
        .map(decode_submission_array)
        .and_then(|submissions| process_submissions(conn, submissions))
        .and_then(|(conn, news)| get_subscribers(conn, subreddit, news))
        .map(|(subscribers, news)| dispatch_news(send, news, subscribers))
}

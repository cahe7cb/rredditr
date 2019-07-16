use redis::r#async::ConnectionLike;
use redis::RedisError;

use futures::future::Future;

use crate::reddit::Submission;
use crate::reddit::RECENT_OFFSET;

pub struct Database<C>(C);

impl<C> Database<C>
where
    C: ConnectionLike + Send + 'static,
{
    pub fn fetch_available_subs(conn: C) -> impl Future<Item = (C, Vec<String>), Error = ()> {
        redis::cmd("SMEMBERS")
            .arg("main/subs")
            .query_async::<_, _>(conn)
            .map_err(|err: RedisError| {
                eprintln!("Failed to fetch available sureddits: {:#?}", err);
            })
    }

    pub fn fetch_subscribers(
        conn: C,
        subreddit: String,
    ) -> impl Future<Item = (C, Vec<i64>), Error = ()> {
        redis::cmd("SMEMBERS")
            .arg(format!("subscribers/{}", subreddit))
            .query_async::<_, _>(conn)
            .map_err(move |err: redis::RedisError| {
                eprintln!("Failed to load subscribers for {}: {:#?}", subreddit, err);
            })
    }

    pub fn update_submissions(
        conn: C,
        submissions: Vec<Submission>,
    ) -> impl Future<Item = (C, Vec<Submission>), Error = ()> {
        let now = chrono::Utc::now().timestamp() as f64;
        let pipe = submissions.iter().fold(redis::pipe(), |mut pipe, post| {
            pipe.cmd("SETEX")
                .arg((post.get_key(), 2.0 * RECENT_OFFSET, post.get_tag(now)))
                .to_owned()
        });
        pipe.query_async::<_, ()>(conn)
            .and_then(|(conn, _)| Ok((conn, submissions)))
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed to update posts: {:#?}", err);
            })
    }

    pub fn fetch_submissions_status(
        conn: C,
        keys: Vec<String>,
    ) -> impl Future<Item = (C, Vec<redis::Value>), Error = ()> {
        redis::cmd("MGET")
            .arg(keys)
            .query_async::<_, Vec<redis::Value>>(conn)
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed retrieving posts status: {:#?}", err);
            })
    }

    pub fn delete_subscriber(
        conn: C,
        target: &String,
        id: i64,
    ) -> impl Future<Item = (C, ()), Error = ()> {
        redis::cmd("SREM")
            .arg(format!("subscribers/{}", target))
            .arg(id)
            .query_async::<_, ()>(conn)
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed to unsubscribe from subreddit: {:#?}", err);
            })
    }

    pub fn fetch_is_sub_available(
        conn: C,
        target: &String,
    ) -> impl Future<Item = (C, bool), Error = ()> {
        redis::cmd("SISMEMBER")
            .arg(format!("main/subs"))
            .arg(target)
            .query_async::<_, bool>(conn)
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed checking for existence of subreddit: {:#?}", err);
            })
            .and_then(|(conn, is_member)| Ok((conn, is_member)))
    }

    pub fn insert_subscriber(
        conn: C,
        target: &String,
        id: i64,
    ) -> impl Future<Item = (C, ()), Error = ()> {
        redis::cmd("SADD")
            .arg(format!("subscribers/{}", target))
            .arg(id)
            .query_async::<_, ()>(conn)
            .map_err(|err: redis::RedisError| {
                eprintln!("Failed to subscribe to: {:#?}", err);
            })
    }
}

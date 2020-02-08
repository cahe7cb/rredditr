use redis::aio::ConnectionLike;
use redis::RedisError;

use crate::reddit::Submission;
use crate::reddit::RECENT_OFFSET;

pub struct Database<C>(C);

impl<C> Database<C>
where
    C: ConnectionLike + Send + 'static,
{
    pub async fn queue_message(
        conn: &mut C,
        str : String
    ) -> Result<(), RedisError> {
        redis::cmd("LPUSH")
            .arg("main/messages")
            .arg(str)
            .query_async::<_, _>(conn)
            .await
    }
    pub async fn fetch_available_subs(
        conn: &mut C
    ) -> Result<Vec<String>, RedisError> {
        redis::cmd("SMEMBERS")
            .arg("main/subs")
            .query_async::<_, _>(conn)
            .await
    }

    pub async fn fetch_submissions_status(
        conn: &mut C,
        keys: Vec<String>,
    ) -> Result<Vec<redis::Value>, RedisError> {
        redis::cmd("MGET")
            .arg(keys)
            .query_async::<_, Vec<redis::Value>>(conn)
            .await
    }

    pub async fn fetch_subscribers(
        conn: &mut C,
        subreddit: &String,
    ) -> Result<Vec<i64>, RedisError> {
        let meuovo = format!("subscribers/{}", subreddit);
        redis::cmd("SMEMBERS")
            .arg(meuovo)
            .query_async::<_, _>(conn)
            .await
    }

    pub async fn update_submissions(
        conn: &mut C,
        submissions: Vec<Submission>,
    ) -> Result<(), RedisError> {
        let now = chrono::Utc::now().timestamp() as f64;
        let pipe = submissions.iter().fold(redis::pipe(), |mut pipe, post| {
            pipe.cmd("SETEX")
                .arg((post.get_key(), (2.0 * RECENT_OFFSET) as i64, post.get_tag(now)))
                .to_owned()
        });
        pipe.query_async::<_, ()>(conn).await
    }
}


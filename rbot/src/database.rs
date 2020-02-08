use redis::aio::ConnectionLike;
use redis::RedisError;

use futures::future::Future;

pub struct Database<C>(C);

impl<C> Database<C>
where
    C: ConnectionLike + Send + 'static,
{
    pub fn fetch_available_subs(conn: C) -> impl Future<Item = (C, Vec<String>), Error = RedisError> {
        redis::cmd("SMEMBERS")
            .arg("main/subs")
            .query_async::<_, _>(conn)
    }

    pub fn delete_subscriber(
        conn: C,
        target: &String,
        id: i64,
    ) -> impl Future<Item = (C, ()), Error = RedisError> {
        redis::cmd("SREM")
            .arg(format!("subscribers/{}", target))
            .arg(id)
            .query_async::<_, ()>(conn)
    }

    pub fn fetch_is_sub_available(
        conn: C,
        target: &String,
    ) -> impl Future<Item = (C, bool), Error = RedisError> {
        redis::cmd("SISMEMBER")
            .arg(format!("main/subs"))
            .arg(target)
            .query_async::<_, bool>(conn)
            .and_then(|(conn, is_member)| Ok((conn, is_member)))
    }

    pub fn insert_subscriber(
        conn: C,
        target: &String,
        id: i64,
    ) -> impl Future<Item = (C, ()), Error = RedisError> {
        redis::cmd("SADD")
            .arg(format!("subscribers/{}", target))
            .arg(id)
            .query_async::<_, ()>(conn)
    }
}

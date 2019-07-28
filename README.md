# Rredditr
### Rust Reddit reader

A simple Telegram bot written in Rust that allows for subscribers to receive new Reddit posts directly in Telegram.

### Building

Use the `cargo` build system for development and release builds.

See the Docker and `docker-compose` configurations for container deployment.

The Redis database is used to keep track of users/subscribers of a given subreddit as well as new posts.

### Running

Make sure to set proper values for `TELEGRAM_BOT_TOKEN` and `REDIS_ADDRESS` in the `.env` file. 

On the database, a `set` called `main/subs` should be populated with the subreddits that will be indexed and updated by the bot.

For Docker use `docker-compose run redis redis-cli -h rredditr_redis_1` to connect to the database container.

Example: for `/r/rust` and `/r/programming`.
```
> SADD main/subs rust
> SADD main/subs programming
```

### Usage

- `/subs`: List available subreddits (on the `main/subs` Redis `set`)
- `/sub <sub name>`: Subscribe for new posts of a subreddit
- `/unsub <sub name>`: Unsubscribe from a subreddit

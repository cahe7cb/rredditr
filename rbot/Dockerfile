FROM rust:1.39

RUN mkdir -p /opt/rredditr/rbot

COPY Cargo.toml /opt/rredditr/rbot
COPY src /opt/rredditr/rbot/src

WORKDIR /opt/rredditr/rbot

RUN cargo build --release

ENTRYPOINT ["cargo"]
CMD ["run", "--release", "rredditr-bot"]

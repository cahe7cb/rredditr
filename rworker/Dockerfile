FROM rust:1.39

RUN mkdir -p /opt/rredditr/rworker

COPY Cargo.toml /opt/rredditr/rworker
COPY src /opt/rredditr/rworker/src

WORKDIR /opt/rredditr/rworker

RUN cargo build --release

ENTRYPOINT ["cargo"]
CMD ["run", "--release", "rredditr-worker"]

FROM rust:1.39

RUN mkdir -p /opt/rredditr
RUN mkdir -p /opt/rredditr/src

COPY Cargo.toml /opt/rredditr
COPY src /opt/rredditr/src

WORKDIR /opt/rredditr

RUN cargo build --release

ENTRYPOINT ["cargo"]
CMD ["run", "--release", "rredditr"]

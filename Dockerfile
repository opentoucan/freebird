FROM docker.io/library/rust:1.83.0-bookworm as build

WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY ./src ./src

RUN apt update && apt install -y \
    pkg-config \
    cmake \
    libssl-dev

RUN cargo build --release

FROM docker.io/library/rust:1.83.0-slim-bookworm as final

RUN apt update && apt install -y \
    catatonit \
    libopus-dev \
    libssl-dev \
    yt-dlp

COPY --from=build /app/target/release /app
COPY --chmod=755 ./entrypoint.sh /entrypoint.sh
USER nobody:nogroup

ENTRYPOINT ["catatonit", "--", "/entrypoint.sh"]
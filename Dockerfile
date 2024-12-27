FROM docker.io/library/rust:1.83.0-bookworm as build

WORKDIR /app
COPY Cargo.toml Cargo.toml
COPY ./src ./src

RUN apt update && apt install -y \
    cmake

RUN cargo build --release

FROM docker.io/library/rust:1.83.0-slim-bookworm as final

RUN apt update && apt install -y \
    catatonit \
    libopus-dev \
    python3 \
    curl

COPY --from=build /app/target/release /app
COPY --chmod=755 ./entrypoint.sh /entrypoint.sh

RUN curl -L https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp -o /usr/local/bin/yt-dlp
RUN chmod a+rx /usr/local/bin/yt-dlp

USER nobody:nogroup

ENTRYPOINT ["catatonit", "--", "/entrypoint.sh"]

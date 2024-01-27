FROM rust as server
RUN cargo install cargo-watch 
WORKDIR /app
VOLUME [ "/data", "/app"]
CMD cargo watch -x 'run --release -- /data/map.osm.pbf'



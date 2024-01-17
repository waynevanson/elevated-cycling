# The Open Elevation API we use to calculate how high segments of a path are.
# We need to bootstrap so that we download the data and run the processing steps for the API to understand.
FROM openelevation/open-elevation as open-elevation
WORKDIR /code
RUN ./download-srtm-data.sh
RUN ./create-dataset.sh
RUN command: stdbuf -i0 -o0 -e0 python3 server.python3

# Download planet.osm.pbf for map data that we can traverse.
# If you already have this file you can skip the download and copy the file to `/data.osm.pbf`.
# https://download.geofabrik.de/
FROM debian as planet-osm
RUN apt-get update
RUN apt-get install -y wget
RUN \
    wget 'https://download.geofabrik.de/australia-oceania/australia-latest.osm.pbf' \
    --output-document '/data/map.osm.pbf' \
    --no-verbose --show-progress --progress=dot:giga:noscroll \
    --continue 

FROM rust as server-build
WORKDIR /app/
RUN \
    cargo install --frozen && \
    cargo build --release elevation

FROM rust as server
WORKDIR /app
VOLUME [ "/data", "/app"]
CMD cargo watch -x run --release -- /data/map.osm.pbf



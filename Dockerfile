# The Open Elevation API we use to calculate how high segments of a path are.
# We need to bootstrap so that we download the data and run the processing steps for the API to understand.
FROM openelevation/open-elevation as open-elevation
WORKDIR /code
RUN ./download-srtm-data.sh
RUN ./create-dataset.sh

# Download planet.osm.pbf for map data that we can traverse.
FROM debian as planet-osm
RUN apt-get update
RUN apt-get install -y curl
RUN curl 'https://download.bbbike.org/osm/planet/planet-latest.osm.pbf' -O '/data.osm.pbf'

FROM rust as server-build
WORKDIR /app/
RUN \
    cargo install --frozen && \
    cargo build --release elevation

FROM rust as server
VOLUME [ "/data", "/app"]
CMD cargo watch -x run --release elevation -- /data.osm.pbf



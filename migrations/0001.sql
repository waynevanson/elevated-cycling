CREATE TABLE nodes (
    -- Same as OSM node_id (i64)
    id BIGINT PRIMARY KEY,

    -- 1. (x, y, NULL) -> Points without elevation
    -- 2. (x, y, z) -> Points with elevation
    x DOUBLE PRECISION NOT NULL
    y DOUBLE PRECISION NOT NULL
    z DOUBLE PRECISION NULL
);

CREATE INDEX index_nodes_point
ON nodes (point);

CREATE TABLE graph (
    source_id BIGINT NOT NULL,
    target_id BIGINT NOT NULL,
    PRIMARY KEY (source_id, target_id),
    CHECK (source_id != target_id),
    CONSTRAINT graph_source_id_fkey FOREIGN KEY (source_id) REFERENCES nodes (id),
    CONSTRAINT graph_target_id_fkey FOREIGN KEY (target_id) REFERENCES nodes (id),
);

CREATE INDEX index_graph_source
ON graph (source_id);

CREATE INDEX index_graph_target
ON graph (target_id);
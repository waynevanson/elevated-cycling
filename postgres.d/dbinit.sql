-- Nullable fields for storing other data
CREATE TABLE osm_node (
    -- node_id for nodes in `*.osm[.pbf]` maps.
    id BIGINT PRIMARY KEY,
    coord GEOMETRY(POINT, 4326) UNIQUE,
    elevation INTEGER
);

CREATE TABLE osm_node_edge (
    source_node_id BIGINT NOT NULL REFERENCES osm_node(id),
    target_node_id BIGINT NOT NULL REFERENCES osm_node(id),
    PRIMARY KEY (source_node_id, target_node_id)
);

-- The database might be the best place to put the graph logic right?
-- If I don't at least I can pull all coords I want into memory.
CREATE INDEX index_osm_node_edge_source_node_id ON osm_node_edge (source_node_id);
CREATE INDEX index_osm_node_edge_target_node_id ON osm_node_edge (target_node_id);
    
-- GPT
-- Ensures that edges are undirected by putting smallest node numbers on the left
CREATE OR REPLACE FUNCTION enforce_node_order()
RETURNS TRIGGER AS $$
DECLARE
    prev_source_node_id INT;
BEGIN
    IF NEW.source_node_id > NEW.target_node_id THEN
        prev_source_node_id := NEW.source_node_id;
        NEW.source_node_id := NEW.target_node_id;
        NEW.target_node_id := prev_source_node_id;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_osm_node_edge_undirected
BEFORE INSERT ON osm_node_edge
FOR EACH ROW
EXECUTE FUNCTION enforce_node_order();

-- The database contains a list of cyclable ways.

-- For now just pull all the coords in a radius
-- (origin = coord, radius) -> node_ids
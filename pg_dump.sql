--
-- PostgreSQL database dump
--

\restrict zIyURMfpPgVjPOHWle7FPzsPetiHDr17Xvtu8xrh89Su7YKfVCGs2VBtt8yFg8Z

-- Dumped from database version 17.6
-- Dumped by pg_dump version 17.6

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET transaction_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: tiger; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA tiger;


ALTER SCHEMA tiger OWNER TO postgres;

--
-- Name: tiger_data; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA tiger_data;


ALTER SCHEMA tiger_data OWNER TO postgres;

--
-- Name: topology; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA topology;


ALTER SCHEMA topology OWNER TO postgres;

--
-- Name: SCHEMA topology; Type: COMMENT; Schema: -; Owner: postgres
--

COMMENT ON SCHEMA topology IS 'PostGIS Topology schema';


--
-- Name: fuzzystrmatch; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS fuzzystrmatch WITH SCHEMA public;


--
-- Name: EXTENSION fuzzystrmatch; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION fuzzystrmatch IS 'determine similarities and distance between strings';


--
-- Name: postgis; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS postgis WITH SCHEMA public;


--
-- Name: EXTENSION postgis; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION postgis IS 'PostGIS geometry and geography spatial types and functions';


--
-- Name: postgis_tiger_geocoder; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS postgis_tiger_geocoder WITH SCHEMA tiger;


--
-- Name: EXTENSION postgis_tiger_geocoder; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION postgis_tiger_geocoder IS 'PostGIS tiger geocoder and reverse geocoder';


--
-- Name: postgis_topology; Type: EXTENSION; Schema: -; Owner: -
--

CREATE EXTENSION IF NOT EXISTS postgis_topology WITH SCHEMA topology;


--
-- Name: EXTENSION postgis_topology; Type: COMMENT; Schema: -; Owner: 
--

COMMENT ON EXTENSION postgis_topology IS 'PostGIS topology spatial types and functions';


--
-- Name: enforce_node_order(); Type: FUNCTION; Schema: public; Owner: postgres
--

CREATE FUNCTION public.enforce_node_order() RETURNS trigger
    LANGUAGE plpgsql
    AS $$
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
$$;


ALTER FUNCTION public.enforce_node_order() OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: osm_node; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.osm_node (
    id integer NOT NULL,
    coord public.geometry(Point,4326),
    elevation integer
);


ALTER TABLE public.osm_node OWNER TO postgres;

--
-- Name: osm_node_edge; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.osm_node_edge (
    source_node_id integer NOT NULL,
    target_node_id integer NOT NULL
);


ALTER TABLE public.osm_node_edge OWNER TO postgres;

--
-- Data for Name: osm_node; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.osm_node (id, coord, elevation) FROM stdin;
\.


--
-- Data for Name: osm_node_edge; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.osm_node_edge (source_node_id, target_node_id) FROM stdin;
\.


--
-- Data for Name: spatial_ref_sys; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.spatial_ref_sys (srid, auth_name, auth_srid, srtext, proj4text) FROM stdin;
\.


--
-- Data for Name: geocode_settings; Type: TABLE DATA; Schema: tiger; Owner: postgres
--

COPY tiger.geocode_settings (name, setting, unit, category, short_desc) FROM stdin;
\.


--
-- Data for Name: pagc_gaz; Type: TABLE DATA; Schema: tiger; Owner: postgres
--

COPY tiger.pagc_gaz (id, seq, word, stdword, token, is_custom) FROM stdin;
\.


--
-- Data for Name: pagc_lex; Type: TABLE DATA; Schema: tiger; Owner: postgres
--

COPY tiger.pagc_lex (id, seq, word, stdword, token, is_custom) FROM stdin;
\.


--
-- Data for Name: pagc_rules; Type: TABLE DATA; Schema: tiger; Owner: postgres
--

COPY tiger.pagc_rules (id, rule, is_custom) FROM stdin;
\.


--
-- Data for Name: topology; Type: TABLE DATA; Schema: topology; Owner: postgres
--

COPY topology.topology (id, name, srid, "precision", hasz, useslargeids) FROM stdin;
\.


--
-- Data for Name: layer; Type: TABLE DATA; Schema: topology; Owner: postgres
--

COPY topology.layer (topology_id, layer_id, schema_name, table_name, feature_column, feature_type, level, child_id) FROM stdin;
\.


--
-- Name: topology_id_seq; Type: SEQUENCE SET; Schema: topology; Owner: postgres
--

SELECT pg_catalog.setval('topology.topology_id_seq', 1, false);


--
-- Name: osm_node osm_node_coord_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.osm_node
    ADD CONSTRAINT osm_node_coord_key UNIQUE (coord);


--
-- Name: osm_node_edge osm_node_edge_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.osm_node_edge
    ADD CONSTRAINT osm_node_edge_pkey PRIMARY KEY (source_node_id, target_node_id);


--
-- Name: osm_node osm_node_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.osm_node
    ADD CONSTRAINT osm_node_pkey PRIMARY KEY (id);


--
-- Name: index_osm_node_edge_source_node_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_osm_node_edge_source_node_id ON public.osm_node_edge USING btree (source_node_id);


--
-- Name: index_osm_node_edge_target_node_id; Type: INDEX; Schema: public; Owner: postgres
--

CREATE INDEX index_osm_node_edge_target_node_id ON public.osm_node_edge USING btree (target_node_id);


--
-- Name: osm_node_edge trigger_osm_node_edge_undirected; Type: TRIGGER; Schema: public; Owner: postgres
--

CREATE TRIGGER trigger_osm_node_edge_undirected BEFORE INSERT ON public.osm_node_edge FOR EACH ROW EXECUTE FUNCTION public.enforce_node_order();


--
-- Name: osm_node_edge osm_node_edge_source_node_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.osm_node_edge
    ADD CONSTRAINT osm_node_edge_source_node_id_fkey FOREIGN KEY (source_node_id) REFERENCES public.osm_node(id);


--
-- Name: osm_node_edge osm_node_edge_target_node_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.osm_node_edge
    ADD CONSTRAINT osm_node_edge_target_node_id_fkey FOREIGN KEY (target_node_id) REFERENCES public.osm_node(id);


--
-- PostgreSQL database dump complete
--

\unrestrict zIyURMfpPgVjPOHWle7FPzsPetiHDr17Xvtu8xrh89Su7YKfVCGs2VBtt8yFg8Z


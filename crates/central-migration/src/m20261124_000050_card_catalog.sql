-- Public Magic card catalog (oracle search names and printings), populated
-- by the operator (`central-service --cards-ingest-stdin`) and read-only for
-- authenticated sessions (`central_runtime`).
CREATE TABLE central_identity.card_printings (
    scryfall_id uuid PRIMARY KEY,
    oracle_id uuid NOT NULL,
    en_name text NOT NULL,
    printed_name text,
    lang text NOT NULL,
    set_code text NOT NULL,
    set_name text NOT NULL,
    collector_number text NOT NULL,
    released_at date NOT NULL,
    finishes smallint NOT NULL CHECK (finishes BETWEEN 0 AND 7),
    is_ub boolean NOT NULL DEFAULT false,
    has_image boolean NOT NULL DEFAULT true
);
CREATE INDEX card_printings_oracle_lang_idx
    ON central_identity.card_printings (oracle_id, lang, released_at DESC, set_code, collector_number);
CREATE INDEX card_printings_en_name_lower_idx
    ON central_identity.card_printings (pg_catalog.lower(en_name));
REVOKE ALL ON central_identity.card_printings FROM PUBLIC;

CREATE TABLE central_identity.card_search_names (
    oracle_id uuid NOT NULL,
    lang text NOT NULL,
    name text NOT NULL,
    name_lower text NOT NULL,
    en_name text NOT NULL,
    PRIMARY KEY (oracle_id, lang, name_lower)
);
CREATE INDEX card_search_names_lang_lower_idx
    ON central_identity.card_search_names (lang, name_lower text_pattern_ops);
CREATE INDEX card_search_names_en_name_lower_idx
    ON central_identity.card_search_names (pg_catalog.lower(en_name));
REVOKE ALL ON central_identity.card_search_names FROM PUBLIC;

-- Optional write-once composition for original artwork, including existing imported playmats.
CREATE TABLE central_identity.playmat_framing (
    art_id uuid PRIMARY KEY,
    kind text GENERATED ALWAYS AS ('playmat'::text) STORED,
    framing jsonb NOT NULL CHECK (jsonb_typeof(framing)='object' AND octet_length(framing::text)<=4096),
    FOREIGN KEY (art_id,kind) REFERENCES central_identity.advent_art (id,kind) ON DELETE CASCADE
);
REVOKE ALL ON central_identity.playmat_framing FROM PUBLIC;

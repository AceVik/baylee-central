# Provisioning private Advent campaigns

The offline command `central-service --advent-import-stdin` publishes one complete year. Install the matching `central-picture-worker` next to the service executable. The import has no HTTP route. The picture worker receives one still over a pipe. Its environment is empty and its only successful output is the decoded image size.

## Prepare a private directory

Use an operator-owned directory with owner-only filesystem permissions, outside the source checkout, web root, CDN, deployment image and build logs. Stop all writers while importing it. Keep the archive, authoring masters and certification reports in protected storage with an encrypted backup. Run the command from the trusted service installation.

At its root place a UTF-8 `campaign.json` object with these fields.

| Field | Content |
| --- | --- |
| `year` | An integer from 2026 through 2200 |
| `hero` | Image object for the calendar |
| `doors` | Exactly 24 entries with distinct numbers 1 through 24 |

Each entry contains its integer `door`, `kind` (sleeve or playmat), `tier` (standard, special or finale), `art` (image object), `challenge` (relative JSON file path) and `challenge_sha256`. Each image object has a relative `path`, lower-case hexadecimal `sha256` for the exact file and a `title` object with German `de` and English `en` strings. Each challenge file contains a typed `puzzle` and exactly two bilingual `hints`. Protect the manifest and deliver it together with trusted content. Its hash fields detect inconsistent input during staging.

Supply still WebP with a single VP8 or VP8L chunk, at most 16 MiB and 4096 pixels on each side. The worker verifies framing, decodes the entire still and rejects extra chunks, animation and metadata. Titles allow up to 160 UTF-8 bytes per language, hints up to 512. The importer verifies each file digest, bounded puzzle search and the existence of winning and losing lines. Keep solutions outside the import manifest.

## Restrict provisioning

Apply Central migrations, create a dedicated `central_advent_importer` role with ordinary restricted privileges, and apply `ops/advent-importer-grants.sql` as the schema owner. Grant the provisioning login database CONNECT and membership in that role. Keep it separate from the migration and runtime logins. The importer checks its effective rights before staging. The grants let it compare immutable metadata and insert new publication rows. Reading stored image payloads, changing a previous title or awarding an item requires other authority.

Read credentials directly from the operator's secret provider. Pipe a single settings document to standard input. Use absolute `pack_root` and `database.tls` with the same `host`, `port`, `name`, `user`, `password` and `ca_certificate` fields as the migration command. For an isolated developer instance use `database.loopback_fixture` with `port`, `name`, `user` and `password`. This transport always connects to IPv4 loopback. Keep PostgreSQL and audit collectors configured to exclude bound parameters, statement bodies and private error details from logs. Restrict database dumps, replicas and backups to the same audience.

## Verify and recover

Success reports 24 doors and one calendar image. A second import of the identical source verifies the same immutable records. Concurrent imports serialize. A failed read, digest mismatch, puzzle validation failure, SQL conflict or unfinished transaction leaves the previous publication intact. Correct the source or the staging login and rerun the whole command. A changed published reward, date, hint or puzzle is refused. A content correction to a live season requires a separate reviewed migration.

The publication opens on December 1 at 00:00 Europe/Berlin and closes on January 7 at 00:00 Europe/Berlin. Central checks release time and account entitlement when serving content. The source directory can be taken offline once the committed database and backup have been verified.

For a private end-to-end release rehearsal, clone the staging database into an isolated native test fixture called `central_native_test`. Set its database comment to `advent-disposable-clone-v1` and point `CENTRAL_TEST_PROVISIONED_ADVENT_PACK` at the trusted source directory before running the existing native HTTP PostgreSQL suite. Only this marked clone can run the calendar time-shift check. It exercises the SDK, independent synthetic native sessions, automatic inventory delivery and byte equality with each approved local file. Destroy the disposable clone after the check and stop the test service.

## Private ranked catalog

Use `central-service --ranked-cosmetics-import-stdin` with the same stdin settings, picture worker and confined file layout. The DBA also applies `ops/ranked-cosmetics-importer-grants.sql` for the trusted catalog operator. Keep this pack separate from seasonal calendar content. Its root contains `catalog.json` with one `sets` array of 28 objects. Each has integer `league` and `division`, plus the two image objects `sleeve` and `playmat`. Use the image path, SHA-256 and bilingual title fields described above. A playmat can also carry its original composition and multiplayer overrides in `mat_framing`.

| League codes | Required divisions | Finish |
| --- | --- | --- |
| 0 Paper | 0 | Standard |
| 1 Wood through 4 Silver | I, II and III, encoded as 1 to 3 | Standard |
| 5 Gold through 7 Diamond | I through IV, encoded as 1 to 4 | Standard |
| 8 Master and 9 Grandmaster | 0 | Special |
| 10 Champion | 0 | Finale |

Each of the 56 slots uses distinct artwork. The operator validates and commits all 28 pairs in one transaction. Reimporting checks titles, digests, image dimensions, finish and the complete composition. A missing, duplicate or changed slot aborts the transaction. Account rewards follow the independently recorded league and division acquisitions when the matching catalog entries become available.

## Optional playmat composition

A playmat image object may include `mat_framing`. Its `duel` object defines the original one-on-one composition. `focus: [x,y]` locates the motif from the top left of the source. Both coordinates range from 0 to 10000, with 10000 indicating the opposite edge. The optional `safe: [left,top,right,bottom]` marks a positive-area rectangle which contains the focus and should remain visible.

An optional `variants` array holds up to eight exceptions. Each object contains `seats`, `allies`, `sides` and `framing`. These count participating seats, members of the owner's team including the owner, and teams in the game. For two against two the key is (4,2,2). For three against two it is (5,3,2) or (5,2,2) depending on the wearer. A five-player free-for-all uses (5,1,5). Each key appears once. Unlisted layouts inherit the duel composition. Images without framing use centered rendering.

Migration 46 stores one optional, append-only composition per playmat. Reimporting the original complete pack can add its first composition to existing artwork. A later import can omit it or confirm identical coordinates. The artwork ID, image digest, earned item and original bytes stay stable. A conflicting layout or an invalid image role fails the entire import. The operator and runtime grants include the new table. Provision real compositions only from private release material.

Authorized native and gateway HTTP calls can send `baylee-mat-framing: 1` to receive the composition on the existing asset metadata, including successful conditional cache responses. The original response format remains available to clients that omit the header. Metadata follows the same ownership, release or live-table checks as its image.

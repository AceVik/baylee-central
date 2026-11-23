//! Immutable operator provisioning. Dropping an incomplete import rolls back its art and doors.
use crate::StoreError;
use central_api::advent::{DOORS, Kind, MAX_ART_BYTES, Text, puzzle::Puzzle};
use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, Statement,
    TransactionTrait, Value, prelude::Uuid,
};
use sha2::{Digest, Sha256};

/// A private still WebP validated by the isolated picture worker.
pub struct Art {
    /// An earned slot, or None for the calendar illustration.
    pub kind: Option<Kind>,
    /// Visual treatment selected by the operator.
    pub tier: String,
    /// Localized inventory caption.
    pub title: Text,
    /// Decoder-confirmed pixel width and height.
    pub dimensions: [u32; 2],
    /// Original metadata-free bytes.
    pub bytes: Vec<u8>,
}

/// At most one full campaign can be committed by this transaction.
pub struct Import {
    tx: DatabaseTransaction,
    year: i16,
    completed: Option<u8>,
}

fn sql(query: &str, values: Vec<Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, query, values)
}

fn text_ok(text: &Text, limit: usize) -> bool {
    [&text.de, &text.en]
        .into_iter()
        .all(|s| !s.trim().is_empty() && s.len() <= limit && !s.chars().any(char::is_control))
}

async fn store_art(tx: &DatabaseTransaction, art: Art) -> Result<Uuid, StoreError> {
    let [width, height] = art.dimensions;
    if ![width, height]
        .into_iter()
        .all(|side| (1..=4096).contains(&side))
        || art.bytes.is_empty()
        || art.bytes.len() > MAX_ART_BYTES as usize
        || !text_ok(&art.title, 160)
        || !matches!(art.tier.as_str(), "standard" | "special" | "finale")
        || (art.kind.is_none() && art.tier != "standard")
    {
        return Err(StoreError::InvalidConfig);
    }
    let size = i32::try_from(art.bytes.len()).map_err(|_| StoreError::InvalidConfig)?;
    let width = i32::try_from(width).map_err(|_| StoreError::InvalidConfig)?;
    let height = i32::try_from(height).map_err(|_| StoreError::InvalidConfig)?;
    let digest = Sha256::digest(&art.bytes).to_vec();
    let keys = vec![
        digest.into(),
        art.kind.map_or("hero", Kind::as_str).into(),
        art.tier.into(),
        art.title.de.into(),
        art.title.en.into(),
        size.into(),
        width.into(),
        height.into(),
    ];
    let mut values = keys.clone();
    values.push(art.bytes.into());
    tx.execute_raw(sql("INSERT INTO central_identity.advent_art(digest,kind,tier,title_de,title_en,size,width,height,mime,content)
        VALUES($1,$2,$3,$4,$5,$6,$7,$8,'image/webp',$9) ON CONFLICT(digest) DO NOTHING",values)).await.map_err(|_| StoreError::Unavailable)?;
    tx.query_one_raw(sql("SELECT id FROM central_identity.advent_art WHERE digest=$1 AND kind=$2 AND tier=$3 AND title_de=$4
        AND title_en=$5 AND size=$6 AND width=$7 AND height=$8 AND mime='image/webp'",keys)).await
        .map_err(|_|StoreError::Unavailable)?.ok_or(StoreError::InvalidConfig)?.try_get("","id").map_err(|_|StoreError::Unavailable)
}

async fn check_role(tx: &DatabaseTransaction) -> Result<(), StoreError> {
    let ok = tx.query_one_raw(sql(
        "SELECT 1 FROM pg_catalog.pg_roles r WHERE r.rolname=current_user AND NOT
         (r.rolsuper OR r.rolcreaterole OR r.rolcreatedb OR r.rolreplication OR r.rolbypassrls)
         AND pg_has_role(current_user,'central_advent_importer','USAGE')
         AND NOT has_schema_privilege(current_user,'central_identity','CREATE')
         AND NOT has_column_privilege(current_user,'central_identity.advent_art','content','SELECT')
         AND NOT EXISTS (SELECT 1 FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
             WHERE n.nspname='central_identity' AND c.relkind IN ('r','p') AND (
               has_table_privilege(current_user,c.oid,'DELETE,TRUNCATE,TRIGGER,MAINTAIN')
               OR has_any_column_privilege(current_user,c.oid,'UPDATE')
               OR (c.relname NOT IN ('advent_art','advent_campaigns','advent_doors')
                 AND has_any_column_privilege(current_user,c.oid,'SELECT,INSERT,REFERENCES'))))",
        vec![])).await.map_err(|_|StoreError::UnsafeRole)?;
    ok.map(|_| ()).ok_or(StoreError::UnsafeRole)
}

impl Import {
    /// Stage the hero and the calendar's release and closing instants.
    ///
    /// # Errors
    /// Refuses invalid input, an incompatible previous publication or a database failure.
    pub async fn begin(db: &DatabaseConnection, year: u16, hero: Art) -> Result<Self, StoreError> {
        if !(2026..=2200).contains(&year) || hero.kind.is_some() {
            return Err(StoreError::InvalidConfig);
        }
        let year = i16::try_from(year).map_err(|_| StoreError::InvalidConfig)?;
        let tx = db.begin().await.map_err(|_| StoreError::Unavailable)?;
        check_role(&tx).await?;
        // Imports run rarely. Serialize all years, even if they reuse the same art in a different order.
        tx.query_one_raw(sql(
            "SELECT pg_advisory_xact_lock(1650553465, 1633973861)",
            vec![],
        ))
        .await
        .map_err(|_| StoreError::Unavailable)?;
        let id = store_art(&tx, hero).await?;
        let values = vec![year.into(), id.into()];
        tx.execute_raw(sql(
            "INSERT INTO central_identity.advent_campaigns(year,hero_id,starts_at,closes_at)
            VALUES($1,$2,make_timestamptz($1::smallint::integer,12,1,0,0,0,'Europe/Berlin'),
                         make_timestamptz($1::smallint::integer+1,1,7,0,0,0,'Europe/Berlin'))
            ON CONFLICT(year) DO NOTHING",
            values.clone(),
        ))
        .await
        .map_err(|_| StoreError::Unavailable)?;
        tx.query_one_raw(sql(
            "SELECT year FROM central_identity.advent_campaigns WHERE year=$1 AND hero_id=$2
            AND starts_at=make_timestamptz($1::smallint::integer,12,1,0,0,0,'Europe/Berlin')
            AND closes_at=make_timestamptz($1::smallint::integer+1,1,7,0,0,0,'Europe/Berlin')",
            values,
        ))
        .await
        .map_err(|_| StoreError::Unavailable)?
        .ok_or(StoreError::InvalidConfig)?;
        Ok(Self {
            tx,
            year,
            completed: Some(0),
        })
    }

    /// Validate and stage the next numbered puzzle and cosmetic.
    ///
    /// # Errors
    /// Rejects malformed puzzles, missing or changed material and database failures.
    pub async fn door(
        &mut self,
        number: u8,
        puzzle: &Puzzle,
        hints: [Text; 2],
        art: Art,
    ) -> Result<(), StoreError> {
        // A failed door poisons the transaction even when an operator API caller catches
        // the error and attempts to continue with corrected content on the same handle.
        let previous = self.completed.take().ok_or(StoreError::InvalidConfig)?;
        if number != previous + 1
            || number > DOORS
            || art.kind.is_none()
            || !hints.iter().all(|h| text_ok(h, 512))
        {
            return Err(StoreError::InvalidConfig);
        }
        let study = puzzle
            .study(100_000)
            .map_err(|_| StoreError::InvalidConfig)?;
        if study.wins == 0 || study.losses == 0 {
            return Err(StoreError::InvalidConfig);
        }
        let encoded = central_api::encode(puzzle).map_err(|_| StoreError::InvalidConfig)?;
        if encoded.len() > 32_768 {
            return Err(StoreError::InvalidConfig);
        }
        let json = String::from_utf8(encoded).map_err(|_| StoreError::InvalidConfig)?;
        let art_id = store_art(&self.tx, art).await?;
        let [one, two] = hints;
        let values = vec![
            self.year.into(),
            i16::from(number).into(),
            art_id.into(),
            json.into(),
            one.de.into(),
            one.en.into(),
            two.de.into(),
            two.en.into(),
        ];
        self.tx.execute_raw(sql("INSERT INTO central_identity.advent_doors(year,door,art_id,puzzle,hint1_de,hint1_en,hint2_de,hint2_en)
            VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT(year,door) DO NOTHING",values.clone())).await.map_err(|_|StoreError::Unavailable)?;
        self.tx.query_one_raw(sql("SELECT door FROM central_identity.advent_doors
            WHERE year=$1 AND door=$2 AND art_id=$3 AND puzzle=$4 AND hint1_de=$5 AND hint1_en=$6 AND hint2_de=$7 AND hint2_en=$8",values)).await
            .map_err(|_|StoreError::Unavailable)?.ok_or(StoreError::InvalidConfig)?;
        self.completed = Some(number);
        Ok(())
    }

    /// Publish every door and all assets together.
    ///
    /// # Errors
    /// Rejects incomplete campaigns and commit failures.
    pub async fn finish(self) -> Result<(), StoreError> {
        if self.completed != Some(DOORS) {
            return Err(StoreError::InvalidConfig);
        }
        self.tx.commit().await.map_err(|_| StoreError::Unavailable)
    }
}

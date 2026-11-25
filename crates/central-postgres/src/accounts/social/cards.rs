//! Authenticated card autocomplete and printing queries backed by the
//! operator-ingested Scryfall catalog (`card_search_names`, `card_printings`).
//!
//! Search strictly admits the client's UI language and English (`en`), and
//! suppresses `scryfall_id` on Universes Beyond or image-less printings so
//! browsers never fetch restricted art.

use std::collections::BTreeSet;
use std::fmt::Write;

use central_api::social::{
    CardPrinting, CardSuggestion, FINISH_ETCHED, FINISH_FOIL, FINISH_NONFOIL, MAX_CARD_FILTER,
    MAX_CARD_PRINTINGS, MAX_CARD_QUERY, MAX_CARD_RESULTS,
};
use sea_orm::{ConnectionTrait, QueryResult, Value, prelude::Uuid};

use super::{Error, statement, unavailable};

fn scryfall_lang(tag: &str) -> &'static str {
    match tag.trim().to_ascii_lowercase().as_str() {
        "de" => "de",
        "fr" => "fr",
        "it" => "it",
        "es" => "es",
        "pt" | "pt-br" | "pt_br" => "pt",
        "ru" => "ru",
        "ja" => "ja",
        "ko" => "ko",
        "zh-hans" | "zh-cn" | "zhs" => "zhs",
        "zh-hant" | "zh-tw" | "zht" => "zht",
        _ => "en",
    }
}

fn clean_text(raw: &str, max_chars: usize) -> Result<String, Error> {
    let trimmed = raw.trim();
    if trimmed.chars().count() > max_chars || trimmed.chars().any(char::is_control) {
        return Err(Error::Rejected);
    }
    Ok(trimmed.to_owned())
}

fn clean_opt_filter(raw: Option<&str>) -> Result<Option<String>, Error> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = clean_text(raw, MAX_CARD_FILTER)?;
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(trimmed.to_lowercase()))
    }
}

fn finish_bit(raw: Option<&str>) -> Result<Option<i16>, Error> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let trimmed = clean_text(raw, MAX_CARD_FILTER)?;
    if trimmed.is_empty() {
        return Ok(None);
    }
    match trimmed.to_ascii_lowercase().as_str() {
        "nonfoil" | "normal" | "plain" => Ok(Some(i16::from(FINISH_NONFOIL))),
        "foil" => Ok(Some(i16::from(FINISH_FOIL))),
        "etched" => Ok(Some(i16::from(FINISH_ETCHED))),
        _ => Err(Error::Rejected),
    }
}

fn escape_like(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    for ch in text.chars() {
        match ch {
            '\\' | '%' | '_' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

fn has_word_start_match(haystack: &str, needle: &str) -> bool {
    let mut offset = 0;
    while let Some(rel) = haystack[offset..].find(needle) {
        let start = offset + rel;
        if start == 0 {
            return true;
        }
        if haystack[..start]
            .chars()
            .next_back()
            .is_some_and(|ch| !ch.is_alphanumeric())
        {
            return true;
        }
        let step = haystack[start..].chars().next().map_or(1, char::len_utf8);
        offset = start + step;
    }
    false
}

fn printing_from_row(row: &QueryResult) -> Result<CardPrinting, Error> {
    let scryfall_id: Uuid = row.try_get("", "scryfall_id").map_err(unavailable)?;
    let set_code: String = row.try_get("", "set_code").map_err(unavailable)?;
    let set_name: String = row.try_get("", "set_name").map_err(unavailable)?;
    let collector_number: String = row.try_get("", "collector_number").map_err(unavailable)?;
    let lang: String = row.try_get("", "lang").map_err(unavailable)?;
    let finishes_i16: i16 = row.try_get("", "finishes").map_err(unavailable)?;
    let is_ub: bool = row.try_get("", "is_ub").map_err(unavailable)?;
    let has_image: bool = row.try_get("", "has_image").map_err(unavailable)?;
    let finishes = u8::try_from(finishes_i16).unwrap_or(0) & 0x07;
    Ok(CardPrinting {
        scryfall_id: (!is_ub && has_image).then(|| scryfall_id.hyphenated().to_string()),
        set_code,
        set_name,
        collector_number,
        lang,
        finishes,
        ub: is_ub,
    })
}

struct Filters {
    set_prefix: Option<String>,
    set_contains: Option<String>,
    collector_prefix: Option<String>,
    finish_bit: Option<i16>,
}

impl Filters {
    fn from_raw(
        set_filter: Option<&str>,
        collector_filter: Option<&str>,
        finish_filter: Option<&str>,
    ) -> Result<Self, Error> {
        let set = clean_opt_filter(set_filter)?;
        let collector = clean_opt_filter(collector_filter)?;
        let finish_bit = finish_bit(finish_filter)?;
        Ok(Self {
            set_prefix: set.as_deref().map(|s| format!("{}%", escape_like(s))),
            set_contains: set.as_deref().map(|s| format!("%{}%", escape_like(s))),
            collector_prefix: collector.as_deref().map(|s| format!("{}%", escape_like(s))),
            finish_bit,
        })
    }

    fn append_sql(&self, sql: &mut String, args: &mut Vec<Value>, alias: &str) {
        if let (Some(prefix), Some(contains)) = (&self.set_prefix, &self.set_contains) {
            let p_idx = args.len() + 1;
            args.push(prefix.clone().into());
            let c_idx = args.len() + 1;
            args.push(contains.clone().into());
            let _ = write!(
                sql,
                " AND ({alias}.set_code LIKE ${p_idx} ESCAPE '\\' OR pg_catalog.lower({alias}.set_name) LIKE ${c_idx} ESCAPE '\\')"
            );
        }
        if let Some(col_prefix) = &self.collector_prefix {
            let idx = args.len() + 1;
            args.push(col_prefix.clone().into());
            let _ = write!(
                sql,
                " AND pg_catalog.lower({alias}.collector_number) LIKE ${idx} ESCAPE '\\'"
            );
        }
    }

    fn append_finish_order(&self, sql: &mut String, args: &mut Vec<Value>, alias: &str) {
        if let Some(bit) = self.finish_bit {
            let idx = args.len() + 1;
            args.push(bit.into());
            let _ = write!(
                sql,
                "CASE WHEN ({alias}.finishes & ${idx}) <> 0 THEN 0 ELSE 1 END, "
            );
        }
    }
}

async fn pick_preview(
    db: &impl ConnectionTrait,
    oracle_id: Uuid,
    preferred_lang: &str,
    client_lang: &str,
    filters: &Filters,
) -> Result<Option<CardPrinting>, Error> {
    let mut allowed_langs = vec![client_lang.to_owned()];
    if client_lang != "en" {
        allowed_langs.push("en".to_owned());
    }
    let mut args: Vec<Value> = vec![
        oracle_id.into(),
        allowed_langs.into(),
        preferred_lang.into(),
    ];
    let prefer_non_ub = filters.set_prefix.is_none();
    let mut sql = String::from(
        "SELECT scryfall_id, set_code, set_name, collector_number, lang, finishes, is_ub, has_image
         FROM central_identity.card_printings p
         WHERE p.oracle_id = $1 AND p.lang = ANY($2)",
    );
    filters.append_sql(&mut sql, &mut args, "p");
    sql.push_str(" ORDER BY ");
    if prefer_non_ub {
        sql.push_str("p.is_ub ASC, ");
    }
    filters.append_finish_order(&mut sql, &mut args, "p");
    sql.push_str(
        "p.has_image DESC,
         CASE WHEN p.lang = $3 THEN 0 WHEN p.lang = 'en' THEN 1 ELSE 2 END,
         p.released_at DESC,
         p.set_code ASC,
         p.collector_number ASC
         LIMIT 1",
    );
    let row = db
        .query_one_raw(statement(&sql, args))
        .await
        .map_err(unavailable)?;
    row.as_ref().map(printing_from_row).transpose()
}

struct MatchedRow {
    rank: u8,
    name_len: usize,
    lang_order: u8,
    name_lower: String,
    oracle_id: Uuid,
    name: String,
    en_name: String,
    matched_lang: String,
}

async fn query_candidates(
    db: &impl ConnectionTrait,
    needle: &str,
    client_lang: &str,
    allowed_langs: &[String],
    filters: &Filters,
) -> Result<Vec<MatchedRow>, Error> {
    let escaped = escape_like(needle);
    let prefix = format!("{escaped}%");
    let word_space = format!("% {escaped}%");
    let word_hyphen = format!("%-{escaped}%");
    let contains = format!("%{escaped}%");
    let mut args: Vec<Value> = vec![
        allowed_langs.to_vec().into(),
        needle.to_owned().into(),
        prefix.into(),
        word_space.into(),
        word_hyphen.into(),
        contains.into(),
        client_lang.into(),
    ];

    let mut sql = String::from(
        "SELECT s.oracle_id, s.lang, s.name, s.name_lower, s.en_name
         FROM central_identity.card_search_names s
         WHERE s.lang = ANY($1)
           AND s.name_lower LIKE $6 ESCAPE '\\'",
    );
    if filters.set_prefix.is_some() || filters.collector_prefix.is_some() {
        sql.push_str(
            " AND EXISTS (SELECT 1 FROM central_identity.card_printings p
              WHERE p.oracle_id = s.oracle_id AND p.lang = ANY($1)",
        );
        filters.append_sql(&mut sql, &mut args, "p");
        sql.push(')');
    }
    sql.push_str(
        " ORDER BY
            CASE
                WHEN s.name_lower = $2 THEN 0
                WHEN s.name_lower LIKE $3 ESCAPE '\\' THEN 1
                WHEN s.name_lower LIKE $4 ESCAPE '\\' OR s.name_lower LIKE $5 ESCAPE '\\' THEN 2
                ELSE 3
            END,
            (s.lang != $7)::int,
            pg_catalog.length(s.name),
            s.name_lower
          LIMIT 64",
    );

    let rows = db
        .query_all_raw(statement(&sql, args))
        .await
        .map_err(unavailable)?;

    let mut candidates = Vec::with_capacity(rows.len());
    for row in rows {
        let oracle_id: Uuid = row.try_get("", "oracle_id").map_err(unavailable)?;
        let lang: String = row.try_get("", "lang").map_err(unavailable)?;
        let name: String = row.try_get("", "name").map_err(unavailable)?;
        let name_lower: String = row.try_get("", "name_lower").map_err(unavailable)?;
        let en_name: String = row.try_get("", "en_name").map_err(unavailable)?;
        let rank = if name_lower == needle {
            0
        } else if name_lower.starts_with(needle) {
            1
        } else if has_word_start_match(&name_lower, needle) {
            2
        } else {
            3
        };
        let lang_order = u8::from(lang != client_lang);
        candidates.push(MatchedRow {
            rank,
            name_len: name.len(),
            lang_order,
            name_lower,
            oracle_id,
            name,
            en_name,
            matched_lang: lang,
        });
    }

    candidates.sort_by(|a, b| {
        (a.rank, a.lang_order, a.name_len, &a.name_lower).cmp(&(
            b.rank,
            b.lang_order,
            b.name_len,
            &b.name_lower,
        ))
    });
    Ok(candidates)
}

pub(super) async fn suggest(
    db: &impl ConnectionTrait,
    query: &str,
    lang_tag: &str,
    set_filter: Option<&str>,
    collector_filter: Option<&str>,
    finish_filter: Option<&str>,
) -> Result<Vec<CardSuggestion>, Error> {
    let trimmed = clean_text(query, MAX_CARD_QUERY)?;
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let client_lang = scryfall_lang(lang_tag);
    let needle = trimmed.to_lowercase();
    let filters = Filters::from_raw(set_filter, collector_filter, finish_filter)?;

    let mut allowed_langs = vec![client_lang.to_owned()];
    if client_lang != "en" {
        allowed_langs.push("en".to_owned());
    }
    let candidates = query_candidates(db, &needle, client_lang, &allowed_langs, &filters).await?;

    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for item in candidates {
        if !seen.insert(item.oracle_id) {
            continue;
        }
        let preview = pick_preview(
            db,
            item.oracle_id,
            &item.matched_lang,
            client_lang,
            &filters,
        )
        .await?;
        out.push(CardSuggestion {
            name: item.name,
            en_name: item.en_name,
            matched_lang: item.matched_lang,
            preview,
        });
        if out.len() >= MAX_CARD_RESULTS {
            break;
        }
    }
    Ok(out)
}

pub(super) async fn printings(
    db: &impl ConnectionTrait,
    name: &str,
    lang_tag: &str,
    set_filter: Option<&str>,
    collector_filter: Option<&str>,
    finish_filter: Option<&str>,
) -> Result<(Option<CardSuggestion>, Vec<CardPrinting>), Error> {
    let trimmed = clean_text(name, MAX_CARD_QUERY)?;
    if trimmed.is_empty() {
        return Ok((None, Vec::new()));
    }
    let client_lang = scryfall_lang(lang_tag);
    let needle = trimmed.to_lowercase();
    let filters = Filters::from_raw(set_filter, collector_filter, finish_filter)?;

    let mut allowed_langs = vec![client_lang.to_owned()];
    if client_lang != "en" {
        allowed_langs.push("en".to_owned());
    }

    let matched_local = db
        .query_one_raw(statement(
            "SELECT oracle_id, lang, name, en_name
             FROM central_identity.card_search_names
             WHERE lang = ANY($1)
               AND (name_lower = $2 OR pg_catalog.lower(en_name) = $2)
             ORDER BY (name_lower = $2) DESC, (lang = $3) DESC, (lang = 'en') DESC
             LIMIT 1",
            vec![
                allowed_langs.into(),
                needle.clone().into(),
                client_lang.into(),
            ],
        ))
        .await
        .map_err(unavailable)?;

    let matched = if let Some(row) = matched_local {
        Some(row)
    } else {
        db.query_one_raw(statement(
            "SELECT oracle_id, lang, name, en_name
             FROM central_identity.card_search_names
             WHERE name_lower = $1
             ORDER BY (lang = 'en') DESC, lang ASC
             LIMIT 1",
            vec![needle.into()],
        ))
        .await
        .map_err(unavailable)?
    };

    let Some(matched) = matched else {
        return Ok((None, Vec::new()));
    };
    let oracle_id: Uuid = matched.try_get("", "oracle_id").map_err(unavailable)?;
    let matched_lang: String = matched.try_get("", "lang").map_err(unavailable)?;
    let card_name: String = matched.try_get("", "name").map_err(unavailable)?;
    let en_name: String = matched.try_get("", "en_name").map_err(unavailable)?;

    let mut print_langs = vec![matched_lang.clone()];
    if matched_lang != "en" {
        print_langs.push("en".to_owned());
    }
    let mut args: Vec<Value> = vec![
        oracle_id.into(),
        print_langs.into(),
        matched_lang.clone().into(),
    ];
    let mut sql = String::from(
        "SELECT scryfall_id, set_code, set_name, collector_number, lang, finishes, is_ub, has_image
         FROM central_identity.card_printings p
         WHERE p.oracle_id = $1 AND p.lang = ANY($2)",
    );
    filters.append_sql(&mut sql, &mut args, "p");
    sql.push_str(" ORDER BY ");
    if filters.set_prefix.is_none() {
        sql.push_str("p.is_ub ASC, ");
    }
    filters.append_finish_order(&mut sql, &mut args, "p");
    let _ = write!(
        sql,
        "p.has_image DESC,
         CASE WHEN p.lang = $3 THEN 0 WHEN p.lang = 'en' THEN 1 ELSE 2 END,
         p.released_at DESC,
         p.set_code ASC,
         p.collector_number ASC
         LIMIT {MAX_CARD_PRINTINGS}"
    );

    let rows = db
        .query_all_raw(statement(&sql, args))
        .await
        .map_err(unavailable)?;
    let mut list = Vec::with_capacity(rows.len());
    for row in &rows {
        list.push(printing_from_row(row)?);
    }
    let card = CardSuggestion {
        name: card_name,
        en_name,
        matched_lang,
        preview: list.first().cloned(),
    };
    Ok((Some(card), list))
}

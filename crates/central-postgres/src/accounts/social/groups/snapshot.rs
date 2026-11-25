use super::{ConnectionTrait, Error, Uuid, person, statement, unavailable};
use central_api::groups::{Group, Groups, Invite, Member};
use central_api::picture::PictureKey;
use sea_orm::QueryResult;
use std::collections::BTreeMap;

fn picture_key(row: &QueryResult) -> Result<Option<PictureKey>, Error> {
    let raw: Option<Vec<u8>> = row.try_get("", "picture_key").map_err(unavailable)?;
    raw.map(|bytes| bytes.try_into().map_err(unavailable))
        .transpose()
}

pub(super) async fn read(db: &impl ConnectionTrait, me: Uuid) -> Result<Groups, Error> {
    // One statement gives a consistent snapshot of all memberships and invitations,
    // bounded by 20 groups × 50 members. Invitees see only the group profile and inviter.
    let rows = db
        .query_all_raw(statement(
            "WITH mine AS (
                SELECT g.id, g.name, g.description, g.owner,
                       gp.key AS picture_key,
                       gp.sequence IS NOT NULL AS picture_moving,
                       mine.accepted AS joined, mine.inviter, mine.created_at
                FROM central_identity.chat_members mine
                JOIN central_identity.chat_groups g ON g.id = mine.group_id
                LEFT JOIN central_identity.chat_group_pictures gp ON gp.group_id = g.id
                WHERE mine.account_id = $1
            )
            SELECT g.id, g.name, g.description, g.picture_key, g.picture_moving,
                g.owner = m.account_id AS owner,
                (m.admin OR g.owner = m.account_id) AS admin,
                true AS joined, m.accepted,
                a.tag, a.display_name, floor(extract(epoch FROM m.created_at) * 1000)::bigint AS at
            FROM mine g
            JOIN central_identity.chat_members m ON m.group_id = g.id
            JOIN central_identity.accounts a ON a.id = m.account_id
            WHERE g.joined
            UNION ALL
            SELECT g.id, g.name, g.description, g.picture_key, g.picture_moving,
                false AS owner,
                false AS admin,
                false AS joined, false AS accepted,
                a.tag, a.display_name, floor(extract(epoch FROM g.created_at) * 1000)::bigint AS at
            FROM mine g
            JOIN central_identity.accounts a ON a.id = g.inviter
            WHERE NOT g.joined
            ORDER BY id, at, tag LIMIT 1001",
            vec![me.into()],
        ))
        .await
        .map_err(unavailable)?;
    if rows.len() > 1000 {
        return Err(Error::Unavailable);
    }
    let mut groups = BTreeMap::<Uuid, Group>::new();
    let mut invites = Vec::new();
    for row in &rows {
        let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
        let name: String = row.try_get("", "name").map_err(unavailable)?;
        let description: String = row.try_get("", "description").map_err(unavailable)?;
        let picture_key = picture_key(row)?;
        let picture_moving: bool = row.try_get("", "picture_moving").map_err(unavailable)?;
        if row.try_get::<bool>("", "joined").map_err(unavailable)? {
            let group = groups.entry(id).or_insert_with(|| Group {
                id: id.to_string(),
                name,
                description,
                picture_key,
                picture_moving,
                members: Vec::new(),
                invited: Vec::new(),
            });
            if row.try_get::<bool>("", "accepted").map_err(unavailable)? {
                group.members.push(Member {
                    person: person(row)?,
                    owner: row.try_get("", "owner").map_err(unavailable)?,
                    admin: row.try_get("", "admin").map_err(unavailable)?,
                });
            } else {
                group.invited.push(person(row)?);
            }
        } else {
            let at: i64 = row.try_get("", "at").map_err(unavailable)?;
            invites.push(Invite {
                group: id.to_string(),
                name,
                description,
                picture_key,
                picture_moving,
                inviter: person(row)?,
                at: u64::try_from(at).map_err(unavailable)?,
            });
        }
    }
    if groups.len() + invites.len() > 20 {
        return Err(Error::Unavailable);
    }
    Ok(Groups {
        groups: groups.into_values().collect(),
        invites,
    })
}

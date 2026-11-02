use super::{ConnectionTrait, Error, Uuid, person, statement, unavailable};
use central_api::groups::{Group, Groups, Invite, Member};
use std::collections::BTreeMap;

pub(super) async fn read(db: &impl ConnectionTrait, me: Uuid) -> Result<Groups, Error> {
    // One statement gives a consistent snapshot of all memberships and invitations,
    // bounded by 20 groups × 50 members. Invitees see only the inviting person.
    let rows = db.query_all_raw(statement(
        "WITH mine AS (
            SELECT g.id, g.name, g.owner, mine.accepted AS joined, mine.inviter, mine.created_at
            FROM central_identity.chat_members mine JOIN central_identity.chat_groups g ON g.id = mine.group_id WHERE mine.account_id = $1
        )
        SELECT g.id, g.name, g.owner = m.account_id AS owner, true AS joined, m.accepted,
            a.tag, a.display_name, floor(extract(epoch FROM m.created_at) * 1000)::bigint AS at
        FROM mine g JOIN central_identity.chat_members m ON m.group_id = g.id
        JOIN central_identity.accounts a ON a.id = m.account_id WHERE g.joined
        UNION ALL
        SELECT g.id, g.name, false AS owner, false AS joined, false AS accepted,
            a.tag, a.display_name, floor(extract(epoch FROM g.created_at) * 1000)::bigint AS at
        FROM mine g JOIN central_identity.accounts a ON a.id = g.inviter WHERE NOT g.joined
        ORDER BY id, at, tag LIMIT 1001", vec![me.into()],
    )).await.map_err(unavailable)?;
    if rows.len() > 1000 {
        return Err(Error::Unavailable);
    }
    let mut groups = BTreeMap::<Uuid, Group>::new();
    let mut invites = Vec::new();
    for row in &rows {
        let id: Uuid = row.try_get("", "id").map_err(unavailable)?;
        let name: String = row.try_get("", "name").map_err(unavailable)?;
        if row.try_get::<bool>("", "joined").map_err(unavailable)? {
            let group = groups.entry(id).or_insert_with(|| Group {
                id: id.to_string(),
                name,
                members: Vec::new(),
                invited: Vec::new(),
            });
            if row.try_get::<bool>("", "accepted").map_err(unavailable)? {
                group.members.push(Member {
                    person: person(row)?,
                    owner: row.try_get("", "owner").map_err(unavailable)?,
                });
            } else {
                group.invited.push(person(row)?);
            }
        } else {
            let at: i64 = row.try_get("", "at").map_err(unavailable)?;
            invites.push(Invite {
                group: id.to_string(),
                name,
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

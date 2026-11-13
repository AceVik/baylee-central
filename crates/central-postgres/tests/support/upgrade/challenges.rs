//! Add challenge tables over prior accounts without changing those accounts or their sessions.
use super::{refused, scalar, scratch};
use sea_orm::{ConnectionTrait, DatabaseTransaction};

const SQL: &str = include_str!("../../../../central-migration/src/m20261113_000028_challenges.sql");

pub(super) async fn run(tx: &DatabaseTransaction) {
    let accounts = "SELECT count(*)::bigint AS value FROM upgrade_probe.accounts";
    let sessions = "SELECT count(*)::bigint AS value FROM upgrade_probe.sessions";
    let before = (scalar(tx, accounts).await, scalar(tx, sessions).await);
    tx.execute_unprepared(&scratch(SQL)).await.unwrap();
    assert_eq!(
        (scalar(tx, accounts).await, scalar(tx, sessions).await),
        before
    );
    let task = "INSERT INTO upgrade_probe.challenges(kind,starts_at,ends_at,description,digest,publication)
        VALUES('daily',86400,172800,'{}',decode(repeat('01',32),'hex'),'synthetic')";
    refused(tx, &task.replace("86400,172800", "86401,172800")).await;
    tx.execute_unprepared(task).await.unwrap();
    for invalid in [
        "00000000-0000-0000-0000-000000000000",
        "00000000-0000-4000-8000-000000000000",
    ] {
        refused(
            tx,
            &format!("UPDATE upgrade_probe.challenges SET id='{invalid}'"),
        )
        .await;
    }
    assert_eq!(scalar(tx, "SELECT count(*)::bigint AS value FROM upgrade_probe.challenges WHERE uuid_extract_version(id)=7").await, 1);
    tx.execute_unprepared(
        "INSERT INTO upgrade_probe.challenge_participation(account_id,challenge_id)
         SELECT a.id,c.id FROM (SELECT id FROM upgrade_probe.accounts ORDER BY id LIMIT 1) a
         CROSS JOIN upgrade_probe.challenges c;
         INSERT INTO upgrade_probe.challenge_attempts(account_id,challenge_id,kind,request_id,live)
         SELECT account_id,challenge_id,'daily',uuidv7(),true FROM upgrade_probe.challenge_participation;
         INSERT INTO upgrade_probe.challenge_completions(account_id,challenge_id,kind,tactician,first_try)
         SELECT account_id,challenge_id,'daily',true,true FROM upgrade_probe.challenge_participation;"
    ).await.unwrap();
    refused(tx, "INSERT INTO upgrade_probe.challenge_attempts(account_id,challenge_id,kind,request_id,live)
        SELECT account_id,challenge_id,'daily',uuidv7(),true FROM upgrade_probe.challenge_participation").await;
    refused(
        tx,
        "UPDATE upgrade_probe.challenge_attempts SET state='won',finished_at=clock_timestamp()",
    )
    .await;
    refused(tx, "UPDATE upgrade_probe.challenge_attempts SET request_id='00000000-0000-0000-0000-000000000000'").await;
    tx.execute_unprepared("DELETE FROM upgrade_probe.accounts WHERE id IN (SELECT account_id FROM upgrade_probe.challenge_participation)").await.unwrap();
    for table in [
        "challenge_participation",
        "challenge_attempts",
        "challenge_completions",
    ] {
        assert_eq!(
            scalar(
                tx,
                &format!("SELECT count(*)::bigint AS value FROM upgrade_probe.{table}")
            )
            .await,
            0
        );
    }
    assert_eq!(
        scalar(
            tx,
            "SELECT count(*)::bigint AS value FROM upgrade_probe.challenges"
        )
        .await,
        1
    );
}

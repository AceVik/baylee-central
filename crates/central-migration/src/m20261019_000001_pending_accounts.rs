use sea_orm_migration::{prelude::*, sea_orm::ConnectionTrait};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Fixed, reviewed PostgreSQL DDL; no user input or interpolated identifiers.
        manager
            .get_connection()
            .execute_unprepared(include_str!("m20261019_000001_pending_accounts.sql"))
            .await?;
        Ok(())
    }

    // The default down implementation rejects rollback. Deleting reserved identity
    // data requires a reviewed forward migration or an explicit disposable fixture.
}

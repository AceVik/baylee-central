use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(
            "ALTER TABLE central_identity.gateway_tickets ADD COLUMN purpose text NOT NULL DEFAULT 'admit'
             CHECK (purpose IN ('admit', 'export', 'link'));
             ALTER TABLE central_identity.gateway_tickets ALTER COLUMN purpose DROP DEFAULT;",
        ).await?;
        Ok(())
    }
}

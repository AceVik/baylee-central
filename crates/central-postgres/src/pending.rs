use crate::{DevConfig, StoreError, connect_dev};
use sea_orm::{ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, Statement};

mod entity {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "pending_accounts", schema_name = "central_identity")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub created_at: DateTimeWithTimeZone,
        pub expires_at: DateTimeWithTimeZone,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
    impl ActiveModelBehavior for ActiveModel {}
}

/// Public metadata only. Reserving an ID does not activate an account or authorize login.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PendingAccount {
    id: [u8; 16],
    expires_at: i64,
}

impl PendingAccount {
    /// Database-generated `UUIDv7` bytes for a later authenticated enrollment context.
    #[must_use]
    pub const fn id(self) -> [u8; 16] {
        self.id
    }

    /// Reservation expiry as Unix seconds. The service must enforce expiry and quotas.
    #[must_use]
    pub const fn expires_at(self) -> i64 {
        self.expires_at
    }
}

/// Restricted development adapter, deliberately separate from migrations.
/// A future enrollment service must enforce tickets, rate limits and atomic activation.
pub struct PendingAccountStore(DatabaseConnection);

impl PendingAccountStore {
    /// Connect using a non-owner, non-superuser, non-BYPASSRLS runtime role.
    ///
    /// # Errors
    ///
    /// Rejects excessive privileges, missing schema, unsupported version or failed I/O.
    pub async fn connect_dev(config: DevConfig) -> Result<Self, StoreError> {
        let db = connect_dev(config).await?;
        let allowed = db
            .query_one_raw(Statement::from_string(
                DbBackend::Postgres,
                include_str!("pending-role.sql"),
            ))
            .await;
        let accepted = allowed
            .ok()
            .flatten()
            .and_then(|row| row.try_get::<bool>("", "allowed").ok())
            == Some(true);
        if !accepted {
            let _ = db.close().await;
            return Err(StoreError::UnsafeRole);
        }
        Ok(Self(db))
    }

    /// Allocate a pending identifier with INSERT DEFAULT VALUES RETURNING.
    /// There is no caller-supplied ID, account lookup or activation in this API.
    ///
    /// # Errors
    ///
    /// Fails closed on database, constraint or unsupported-ID errors.
    pub async fn reserve(&self) -> Result<PendingAccount, StoreError> {
        let model = entity::Entity::insert(entity::ActiveModel::default())
            .exec_with_returning(&self.0)
            .await
            .map_err(|_| StoreError::Unavailable)?;
        let id = *model.id.as_bytes();
        if id[6] >> 4 != 7 || id[8] >> 6 != 2 {
            return Err(StoreError::Unavailable);
        }
        Ok(PendingAccount {
            id,
            expires_at: model.expires_at.timestamp(),
        })
    }

    /// Close the pool and await connection cleanup.
    ///
    /// # Errors
    ///
    /// Reports generic connection cleanup failure.
    pub async fn close(self) -> Result<(), StoreError> {
        self.0.close().await.map_err(|_| StoreError::Unavailable)
    }
}

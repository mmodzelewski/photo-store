use crate::database::DbPool;
use crate::entity::prelude::{AuthTokens, UserAccounts};
use crate::entity::{
    app_users, auth_tokens, sea_orm_active_enums::Provider, user_accounts, user_keys,
};
use crate::error::{Error, Result};
use crate::ulid::Id;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait};
use tracing::error;

pub(super) struct User {
    pub id: Id,
    pub password: String,
}

pub(super) struct AuthRepository;

impl AuthRepository {
    pub async fn save_auth_token(db: &DbPool, user_id: &Id, auth_token: &str) -> Result<()> {
        let token_id = Id::new();
        auth_tokens::ActiveModel {
            id: Set(uuid::Uuid::from(token_id)),
            user_id: Set(uuid::Uuid::from(*user_id)),
            token: Set(auth_token.to_string()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| {
            error!(error = %e, "Could not save auth token");
            Error::Database
        })?;

        Ok(())
    }

    pub async fn get_by_token(db: &DbPool, auth_token: &str) -> Result<Id> {
        let row = AuthTokens::find()
            .filter(auth_tokens::Column::Token.eq(auth_token))
            .one(db)
            .await
            .map_err(|e| {
                error!(error = %e, "Could not query auth token");
                Error::Database
            })?
            .ok_or_else(|| {
                error!("Auth token not found");
                Error::Database
            })?;

        Ok(Id::from(row.user_id))
    }

    pub(crate) async fn save_keys(
        db: &DbPool,
        user_id: &Id,
        private_key: &str,
        public_key: &str,
    ) -> Result<()> {
        let keys_id = Id::new();
        user_keys::ActiveModel {
            id: Set(uuid::Uuid::from(keys_id)),
            user_id: Set(uuid::Uuid::from(*user_id)),
            private_key: Set(private_key.to_string()),
            public_key: Set(public_key.to_string()),
            ..Default::default()
        }
        .insert(db)
        .await
        .map_err(|e| {
            error!(error = %e, "Could not save user keys");
            Error::Database
        })?;

        Ok(())
    }

    pub(crate) async fn get_private_key(db: &DbPool, user_id: &Id) -> Result<Option<String>> {
        let result = user_keys::Entity::find()
            .filter(user_keys::Column::UserId.eq(uuid::Uuid::from(*user_id)))
            .one(db)
            .await
            .map_err(|e| {
                error!(error = %e, "Could not get private key");
                Error::Database
            })?
            .map(|row| row.private_key);

        Ok(result)
    }

    pub async fn save_user_with_credentials(
        db: &DbPool,
        user_id: &Id,
        username: &str,
        password_hash: &str,
    ) -> Result<()> {
        db.transaction::<_, (), sea_orm::DbErr>(|txn| {
            let user_id = *user_id;
            let username = username.to_string();
            let password_hash = password_hash.to_string();
            Box::pin(async move {
                app_users::ActiveModel {
                    id: Set(uuid::Uuid::from(user_id)),
                    name: Set(Some(username.clone())),
                    ..Default::default()
                }
                .insert(txn)
                .await?;

                let account_id = Id::new();
                user_accounts::ActiveModel {
                    id: Set(uuid::Uuid::from(account_id)),
                    user_id: Set(uuid::Uuid::from(user_id)),
                    account_id: Set(username),
                    password: Set(Some(password_hash)),
                    provider: Set(Provider::Credentials),
                    ..Default::default()
                }
                .insert(txn)
                .await?;

                Ok(())
            })
        })
        .await
        .map_err(|e| {
            error!(error = %e, "Could not save user with credentials");
            Error::Database
        })?;

        Ok(())
    }

    pub async fn get_by_username(db: &DbPool, username: &str) -> Result<User> {
        let account = UserAccounts::find()
            .filter(user_accounts::Column::AccountId.eq(username))
            .filter(user_accounts::Column::Provider.eq(Provider::Credentials))
            .one(db)
            .await
            .map_err(|e| {
                error!(error = %e, "Could not query user account");
                Error::Database
            })?
            .ok_or_else(|| {
                error!("User account not found");
                Error::Database
            })?;

        Ok(User {
            id: Id::from(account.user_id),
            password: account
                .password
                .expect("Password must be set for credentials user"),
        })
    }
}

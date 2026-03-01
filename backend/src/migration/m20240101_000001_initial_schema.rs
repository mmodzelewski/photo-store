use sea_orm_migration::prelude::{sea_query::extension::postgres::Type, *};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // app_user table
        manager
            .create_table(
                Table::create()
                    .table(AppUser::Table)
                    .col(ColumnDef::new(AppUser::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(AppUser::Name).text())
                    .col(
                        ColumnDef::new(AppUser::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // provider enum
        manager
            .create_type(
                Type::create()
                    .as_enum(ProviderEnum::Type)
                    .values([ProviderEnum::Credentials])
                    .to_owned(),
            )
            .await?;

        // user_account table
        manager
            .create_table(
                Table::create()
                    .table(UserAccount::Table)
                    .col(
                        ColumnDef::new(UserAccount::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(UserAccount::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserAccount::AccountId).text().not_null())
                    .col(ColumnDef::new(UserAccount::Password).text())
                    .col(
                        ColumnDef::new(UserAccount::Provider)
                            .custom(ProviderEnum::Type)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(UserAccount::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserAccount::Table, UserAccount::UserId)
                            .to(AppUser::Table, AppUser::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // unique constraint on (account_id, provider)
        manager
            .create_index(
                Index::create()
                    .name("unique_username_provider")
                    .table(UserAccount::Table)
                    .col(UserAccount::AccountId)
                    .col(UserAccount::Provider)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // auth_token table
        manager
            .create_table(
                Table::create()
                    .table(AuthToken::Table)
                    .col(
                        ColumnDef::new(AuthToken::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(AuthToken::UserId).uuid().not_null())
                    .col(
                        ColumnDef::new(AuthToken::Token)
                            .text()
                            .not_null()
                            .unique_key(),
                    )
                    .col(
                        ColumnDef::new(AuthToken::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(AuthToken::Table, AuthToken::UserId)
                            .to(AppUser::Table, AppUser::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // user_keys table
        manager
            .create_table(
                Table::create()
                    .table(UserKey::Table)
                    .col(ColumnDef::new(UserKey::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(UserKey::UserId).uuid().not_null())
                    .col(ColumnDef::new(UserKey::PrivateKey).text().not_null())
                    .col(ColumnDef::new(UserKey::PublicKey).text().not_null())
                    .col(
                        ColumnDef::new(UserKey::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(UserKey::Table, UserKey::UserId)
                            .to(AppUser::Table, AppUser::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // file_state enum
        manager
            .create_type(
                Type::create()
                    .as_enum(FileStateEnum::Type)
                    .values([
                        FileStateEnum::New,
                        FileStateEnum::SyncInProgress,
                        FileStateEnum::Synced,
                    ])
                    .to_owned(),
            )
            .await?;

        // file table
        manager
            .create_table(
                Table::create()
                    .table(File::Table)
                    .col(ColumnDef::new(File::Id).uuid().not_null().primary_key())
                    .col(ColumnDef::new(File::Path).text().not_null())
                    .col(ColumnDef::new(File::Name).text().not_null())
                    .col(
                        ColumnDef::new(File::State)
                            .custom(FileStateEnum::Type)
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(File::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(File::AddedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(File::Sha256).text().not_null())
                    .col(ColumnDef::new(File::OwnerId).uuid().not_null())
                    .col(ColumnDef::new(File::UploaderId).uuid().not_null())
                    .col(ColumnDef::new(File::EncKey).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .from(File::Table, File::OwnerId)
                            .to(AppUser::Table, AppUser::Id),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .from(File::Table, File::UploaderId)
                            .to(AppUser::Table, AppUser::Id),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum AppUser {
    #[sea_orm(iden = "app_users")]
    Table,
    Id,
    Name,
    CreatedAt,
}

#[derive(DeriveIden)]
enum ProviderEnum {
    #[sea_orm(iden = "provider")]
    Type,
    Credentials,
}

#[derive(DeriveIden)]
enum UserAccount {
    #[sea_orm(iden = "user_accounts")]
    Table,
    Id,
    UserId,
    AccountId,
    Password,
    Provider,
    CreatedAt,
}

#[derive(DeriveIden)]
enum AuthToken {
    #[sea_orm(iden = "auth_tokens")]
    Table,
    Id,
    UserId,
    Token,
    CreatedAt,
}

#[derive(DeriveIden)]
enum UserKey {
    #[sea_orm(iden = "user_keys")]
    Table,
    Id,
    UserId,
    PrivateKey,
    PublicKey,
    CreatedAt,
}

#[derive(DeriveIden)]
enum FileStateEnum {
    #[sea_orm(iden = "file_state")]
    Type,
    New,
    SyncInProgress,
    Synced,
}

#[derive(DeriveIden)]
enum File {
    #[sea_orm(iden = "files")]
    Table,
    Id,
    Path,
    Name,
    State,
    CreatedAt,
    AddedAt,
    Sha256,
    OwnerId,
    UploaderId,
    EncKey,
}

create table app_user
(
    id         uuid        not null primary key,
    name       text,
    created_at timestamptz not null default now()
);

create type provider as enum ('Credentials');

create table user_account
(
    id         uuid        not null primary key,
    user_id    uuid        not null references app_user(id),
    account_id text        not null,
    password   text,
    provider   provider    not null,
    created_at timestamptz not null default now()
);

alter table user_account
add constraint unique_username_provider unique (account_id, provider);

create table auth_token
(
    id         uuid        not null primary key,
    user_id    uuid        not null references app_user(id),
    token      text        not null unique,
    created_at timestamptz not null default now()
);

create table user_keys
(
    id              uuid        not null primary key,
    user_id         uuid        not null references app_user(id),
    private_key     text        not null,
    public_key      text        not null,
    created_at      timestamptz not null default now()
);

create type file_state as enum ('New', 'SyncInProgress', 'Synced');

create table file
(
    id          uuid        not null primary key,
    path        text        not null,
    name        text        not null,
    state       file_state  not null,
    created_at  timestamptz not null,
    added_at    timestamptz not null default now(),
    sha256      text        not null,
    owner_id    uuid        not null references app_user(id),
    uploader_id uuid        not null references app_user(id),
    enc_key     text        not null
);

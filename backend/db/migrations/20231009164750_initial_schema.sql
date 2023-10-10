create table file (
    id serial primary key,
    path text not null,
    name text not null,
    uuid uuid not null,
    created_at timestamptz,
    added_at timestamptz not null default now(),
    sha256 text not null
);


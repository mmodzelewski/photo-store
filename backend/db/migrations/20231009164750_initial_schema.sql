create table file (
    id serial primary key,
    name text not null,
    uuid uuid not null,
    added_at timestamp not null default now()
);


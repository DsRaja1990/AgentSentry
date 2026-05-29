-- AgentSentry Postgres schema v0.1
create table if not exists tenant (
    id          text primary key,
    name        text not null,
    plan        text not null default 'dev',
    created_at  timestamptz not null default now()
);

create table if not exists project (
    id          text primary key,
    tenant_id   text not null references tenant(id),
    name        text not null,
    created_at  timestamptz not null default now()
);

create table if not exists agent (
    id            text primary key,
    project_id    text not null references project(id),
    name          text not null,
    framework     text not null default '',
    owner         text not null default '',
    identity_kind text not null default 'api_key',
    identity_ref  text not null default '',
    created_at    timestamptz not null default now()
);

create table if not exists policy (
    id          text primary key,
    project_id  text not null references project(id),
    name        text not null,
    language    text not null default 'rego',
    source      text not null,
    version     integer not null default 1,
    status      text not null default 'enforced',
    created_at  timestamptz not null default now()
);

create table if not exists api_key (
    id          text primary key,
    tenant_id   text not null references tenant(id),
    hashed_key  text not null unique,
    scopes      text[] not null default '{}',
    created_at  timestamptz not null default now(),
    revoked_at  timestamptz
);

create table if not exists audit_event (
    seq         bigserial primary key,
    prev_hash   text not null default '',
    hash        text not null,
    tenant_id   text not null,
    actor       text not null,
    action      text not null,
    target      text not null,
    payload     jsonb not null,
    ts          timestamptz not null default now()
);

create index if not exists idx_audit_tenant_ts on audit_event (tenant_id, ts desc);
create index if not exists idx_policy_project on policy (project_id, status);
create index if not exists idx_agent_project  on agent  (project_id);

package store

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"errors"
	"fmt"
	"time"

	"github.com/agentsentry/agentsentry/control/internal/model"
	"github.com/google/uuid"
	"github.com/jackc/pgx/v5"
	"github.com/jackc/pgx/v5/pgxpool"
)

type Postgres struct {
	Pool *pgxpool.Pool
}

func NewPostgres(ctx context.Context, dsn string) (*Postgres, error) {
	pool, err := pgxpool.New(ctx, dsn)
	if err != nil {
		return nil, fmt.Errorf("pgx pool: %w", err)
	}
	if err := pool.Ping(ctx); err != nil {
		return nil, fmt.Errorf("pg ping: %w", err)
	}
	return &Postgres{Pool: pool}, nil
}

// ---------------------------------------------------------------- Agents

func (p *Postgres) ListAgents(ctx context.Context) ([]model.Agent, error) {
	rows, err := p.Pool.Query(ctx,
		`select id, project_id, name, framework, owner, identity_kind, identity_ref, created_at
		   from agent order by created_at desc`)
	if err != nil { return nil, err }
	defer rows.Close()
	var out []model.Agent
	for rows.Next() {
		var a model.Agent
		if err := rows.Scan(&a.ID, &a.ProjectID, &a.Name, &a.Framework,
			&a.Owner, &a.IdentityKind, &a.IdentityRef, &a.CreatedAt); err != nil {
			return nil, err
		}
		out = append(out, a)
	}
	return out, rows.Err()
}

func (p *Postgres) CreateAgent(ctx context.Context, a model.Agent) (model.Agent, error) {
	if a.ID == ""        { a.ID = "agt_" + uuid.NewString() }
	if a.ProjectID == "" { a.ProjectID = "p_default" }
	a.CreatedAt = time.Now().UTC()
	_, err := p.Pool.Exec(ctx,
		`insert into agent (id, project_id, name, framework, owner, identity_kind, identity_ref, created_at)
		 values ($1,$2,$3,$4,$5,$6,$7,$8)
		 on conflict (id) do update set name=excluded.name, framework=excluded.framework,
			owner=excluded.owner, identity_kind=excluded.identity_kind, identity_ref=excluded.identity_ref`,
		a.ID, a.ProjectID, a.Name, a.Framework, a.Owner, a.IdentityKind, a.IdentityRef, a.CreatedAt)
	return a, err
}

// ---------------------------------------------------------------- Policies

func (p *Postgres) ListPolicies(ctx context.Context) ([]model.Policy, error) {
	rows, err := p.Pool.Query(ctx,
		`select id, project_id, name, language, source, version, status, created_at
		   from policy order by created_at desc`)
	if err != nil { return nil, err }
	defer rows.Close()
	var out []model.Policy
	for rows.Next() {
		var pl model.Policy
		if err := rows.Scan(&pl.ID, &pl.ProjectID, &pl.Name, &pl.Language,
			&pl.Source, &pl.Version, &pl.Status, &pl.CreatedAt); err != nil {
			return nil, err
		}
		out = append(out, pl)
	}
	return out, rows.Err()
}

func (p *Postgres) GetPolicy(ctx context.Context, id string) (model.Policy, error) {
	var pl model.Policy
	err := p.Pool.QueryRow(ctx,
		`select id, project_id, name, language, source, version, status, created_at
		   from policy where id=$1`, id).
		Scan(&pl.ID, &pl.ProjectID, &pl.Name, &pl.Language,
			&pl.Source, &pl.Version, &pl.Status, &pl.CreatedAt)
	if errors.Is(err, pgx.ErrNoRows) { return pl, ErrNotFound }
	return pl, err
}

func (p *Postgres) UpsertPolicy(ctx context.Context, pl model.Policy) (model.Policy, error) {
	if pl.ID == ""        { pl.ID = "pol_" + uuid.NewString() }
	if pl.ProjectID == "" { pl.ProjectID = "p_default" }
	if pl.Language == ""  { pl.Language = "rego" }
	if pl.Status == ""    { pl.Status = "enforced" }
	if pl.Version == 0    { pl.Version = 1 }
	pl.CreatedAt = time.Now().UTC()
	_, err := p.Pool.Exec(ctx,
		`insert into policy (id, project_id, name, language, source, version, status, created_at)
		 values ($1,$2,$3,$4,$5,$6,$7,$8)
		 on conflict (id) do update set
			name=excluded.name, language=excluded.language, source=excluded.source,
			version=policy.version+1, status=excluded.status`,
		pl.ID, pl.ProjectID, pl.Name, pl.Language, pl.Source, pl.Version, pl.Status, pl.CreatedAt)
	return pl, err
}

// ---------------------------------------------------------------- API keys

var ErrNotFound = errors.New("not found")

func HashKey(raw string) string {
	sum := sha256.Sum256([]byte(raw))
	return hex.EncodeToString(sum[:])
}

func (p *Postgres) CreateAPIKey(ctx context.Context, k model.APIKey) (model.APIKey, error) {
	if k.ID == ""       { k.ID = "key_" + uuid.NewString() }
	if k.TenantID == "" { k.TenantID = "t_default" }
	if k.RawKey == ""   { k.RawKey = "sk_" + uuid.NewString() }
	k.HashedKey = HashKey(k.RawKey)
	k.CreatedAt = time.Now().UTC()

	scopesArr := k.Scopes
	if scopesArr == nil { scopesArr = []string{"ingest", "policy_check"} }

	_, err := p.Pool.Exec(ctx,
		`insert into api_key (id, tenant_id, hashed_key, scopes, created_at)
		 values ($1,$2,$3,$4,$5)`,
		k.ID, k.TenantID, k.HashedKey, scopesArr, k.CreatedAt)
	return k, err
}

func (p *Postgres) LookupAPIKey(ctx context.Context, raw string) (model.APIKey, error) {
	hashed := HashKey(raw)
	var k model.APIKey
	err := p.Pool.QueryRow(ctx,
		`select id, tenant_id, hashed_key, scopes, created_at, revoked_at
		   from api_key where hashed_key=$1`, hashed).
		Scan(&k.ID, &k.TenantID, &k.HashedKey, &k.Scopes, &k.CreatedAt, &k.RevokedAt)
	if errors.Is(err, pgx.ErrNoRows) { return k, ErrNotFound }
	return k, err
}

// LookupAPIKeyByHash resolves an api_key by its pre-hashed (hex sha256) form.
// Used by the ingest path so the gateway never forwards a raw key.
func (p *Postgres) LookupAPIKeyByHash(ctx context.Context, hashed string) (model.APIKey, error) {
	var k model.APIKey
	err := p.Pool.QueryRow(ctx,
		`select id, tenant_id, hashed_key, scopes, created_at, revoked_at
		   from api_key where hashed_key=$1`, hashed).
		Scan(&k.ID, &k.TenantID, &k.HashedKey, &k.Scopes, &k.CreatedAt, &k.RevokedAt)
	if errors.Is(err, pgx.ErrNoRows) { return k, ErrNotFound }
	return k, err
}

// ---------------------------------------------------------------- Audit (hash-chained)

func (p *Postgres) AppendAudit(ctx context.Context, tenantID, actor, action, target string, payload []byte) error {
	var prevHash string
	err := p.Pool.QueryRow(ctx,
		`select coalesce((select hash from audit_event order by seq desc limit 1), '')`).
		Scan(&prevHash)
	if err != nil { return err }

	now := time.Now().UTC()
	rec := fmt.Sprintf("%s|%s|%s|%s|%s|%s|%s",
		prevHash, tenantID, actor, action, target, string(payload), now.Format(time.RFC3339Nano))
	sum := sha256.Sum256([]byte(rec))
	hash := hex.EncodeToString(sum[:])

	_, err = p.Pool.Exec(ctx,
		`insert into audit_event (prev_hash, hash, tenant_id, actor, action, target, payload, ts)
		 values ($1,$2,$3,$4,$5,$6,$7,$8)`,
		prevHash, hash, tenantID, actor, action, target, payload, now)
	return err
}

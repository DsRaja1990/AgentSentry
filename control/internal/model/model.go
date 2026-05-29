package model

import "time"

type Tenant struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	Plan      string    `json:"plan"`
	CreatedAt time.Time `json:"created_at"`
}

type Project struct {
	ID        string    `json:"id"`
	TenantID  string    `json:"tenant_id"`
	Name      string    `json:"name"`
	CreatedAt time.Time `json:"created_at"`
}

type Agent struct {
	ID           string    `json:"id"`
	ProjectID    string    `json:"project_id"`
	Name         string    `json:"name"`
	Framework    string    `json:"framework"`
	Owner        string    `json:"owner"`
	IdentityKind string    `json:"identity_kind"` // api_key | spiffe
	IdentityRef  string    `json:"identity_ref"`
	CreatedAt    time.Time `json:"created_at"`
}

type Policy struct {
	ID        string    `json:"id"`
	ProjectID string    `json:"project_id"`
	Name      string    `json:"name"`
	Language  string    `json:"language"` // rego | cedar
	Source    string    `json:"source"`
	Version   int       `json:"version"`
	Status    string    `json:"status"`   // draft | enforced | monitor | retired
	CreatedAt time.Time `json:"created_at"`
}

type APIKey struct {
	ID         string     `json:"id"`
	TenantID   string     `json:"tenant_id"`
	HashedKey  string     `json:"-"`
	Scopes     []string   `json:"scopes"`
	CreatedAt  time.Time  `json:"created_at"`
	RevokedAt  *time.Time `json:"revoked_at,omitempty"`
	RawKey     string     `json:"raw_key,omitempty"` // returned only on creation
}

type Span struct {
	TraceID  string `json:"trace_id"`
	SpanID   string `json:"span_id"`
	ParentID string `json:"parent_id"`

	TenantID  string `json:"tenant_id"`
	ProjectID string `json:"project_id"`
	AgentID   string `json:"agent_id"`

	TS         string `json:"ts"`
	DurationMS uint64 `json:"duration_ms"`

	Kind     string `json:"kind"`
	Name     string `json:"name"`
	Model    string `json:"model"`
	Provider string `json:"provider"`

	InputTokens  uint64  `json:"input_tokens"`
	OutputTokens uint64  `json:"output_tokens"`
	CostUSD      float64 `json:"cost_usd"`

	ToolName           string `json:"tool_name"`
	ToolArgsRedacted   string `json:"tool_args_redacted"`
	ToolResultRedacted string `json:"tool_result_redacted"`

	PolicyDecision string `json:"policy_decision"`
	PolicyID       string `json:"policy_id"`
	PolicyReason   string `json:"policy_reason"`

	GuardrailHits []string          `json:"guardrail_hits"`
	Attributes    map[string]string `json:"attributes"`

	// CallerKeyHash is hex-encoded SHA-256 of the inbound caller's API key.
	// Control plane resolves it to a tenant during ingest; not persisted.
	CallerKeyHash string `json:"caller_key_hash,omitempty"`

	// Streamed indicates the upstream response was SSE. Token counts may be 0
	// if the stream did not include a usage record.
	Streamed   bool   `json:"streamed,omitempty"`
	HTTPStatus uint16 `json:"http_status,omitempty"`
}

type IngestRequest struct {
	Spans []Span `json:"spans"`
}

type IngestResponse struct {
	Accepted uint64 `json:"accepted"`
	Rejected uint64 `json:"rejected"`
}

type PolicyBundle struct {
	Version  uint64   `json:"version"`
	Policies []Policy `json:"policies"`
}

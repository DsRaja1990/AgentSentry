package store

import (
	"context"
	"fmt"
	"strings"
	"time"

	"github.com/ClickHouse/clickhouse-go/v2"
	"github.com/ClickHouse/clickhouse-go/v2/lib/driver"
	"github.com/agentsentry/agentsentry/control/internal/model"
)

type Clickhouse struct {
	Conn driver.Conn
}

func NewClickhouse(ctx context.Context, dsn string) (*Clickhouse, error) {
	opts, err := clickhouse.ParseDSN(dsn)
	if err != nil {
		return nil, fmt.Errorf("parse clickhouse dsn: %w", err)
	}
	conn, err := clickhouse.Open(opts)
	if err != nil {
		return nil, fmt.Errorf("open clickhouse: %w", err)
	}
	if err := conn.Ping(ctx); err != nil {
		return nil, fmt.Errorf("clickhouse ping: %w", err)
	}
	return &Clickhouse{Conn: conn}, nil
}

func (c *Clickhouse) InsertSpans(ctx context.Context, spans []model.Span) error {
	if len(spans) == 0 { return nil }
	batch, err := c.Conn.PrepareBatch(ctx,
		`insert into agent_span
		 (trace_id, span_id, parent_id, tenant_id, project_id, agent_id,
		  ts, duration_ms, kind, name, model, provider,
		  input_tokens, output_tokens, cost_usd,
		  tool_name, tool_args_redacted, tool_result_redacted,
		  policy_decision, policy_id, policy_reason,
		  guardrail_hits, attributes)`)
	if err != nil { return err }
	for _, s := range spans {
		ts, err := time.Parse(time.RFC3339Nano, s.TS)
		if err != nil { ts = time.Now().UTC() }
		if err := batch.Append(
			s.TraceID, s.SpanID, s.ParentID,
			s.TenantID, s.ProjectID, s.AgentID,
			ts, s.DurationMS, s.Kind, s.Name, s.Model, s.Provider,
			s.InputTokens, s.OutputTokens, s.CostUSD,
			s.ToolName, s.ToolArgsRedacted, s.ToolResultRedacted,
			s.PolicyDecision, s.PolicyID, s.PolicyReason,
			s.GuardrailHits, s.Attributes,
		); err != nil {
			return err
		}
	}
	return batch.Send()
}

type TraceFilter struct {
	AgentID  string
	Since    *time.Time
	Until    *time.Time
	Decision string
	Limit    int
}

func (c *Clickhouse) QueryTraces(ctx context.Context, f TraceFilter) ([]model.Span, error) {
	if f.Limit <= 0 || f.Limit > 1000 { f.Limit = 100 }

	var (
		conds []string
		args  []any
	)
	if f.AgentID != "" {
		conds = append(conds, "agent_id = ?"); args = append(args, f.AgentID)
	}
	if f.Decision != "" {
		conds = append(conds, "policy_decision = ?"); args = append(args, f.Decision)
	}
	if f.Since != nil {
		conds = append(conds, "ts >= ?"); args = append(args, *f.Since)
	}
	if f.Until != nil {
		conds = append(conds, "ts <= ?"); args = append(args, *f.Until)
	}
	where := ""
	if len(conds) > 0 { where = "where " + strings.Join(conds, " and ") }

	q := fmt.Sprintf(`
		select trace_id, span_id, parent_id, tenant_id, project_id, agent_id,
		       ts, duration_ms, kind, name, model, provider,
		       input_tokens, output_tokens, cost_usd,
		       tool_name, tool_args_redacted, tool_result_redacted,
		       policy_decision, policy_id, policy_reason,
		       guardrail_hits, attributes
		  from agent_span %s
		  order by ts desc limit %d`, where, f.Limit)

	rows, err := c.Conn.Query(ctx, q, args...)
	if err != nil { return nil, err }
	defer rows.Close()

	var out []model.Span
	for rows.Next() {
		var s model.Span
		var ts time.Time
		if err := rows.Scan(
			&s.TraceID, &s.SpanID, &s.ParentID,
			&s.TenantID, &s.ProjectID, &s.AgentID,
			&ts, &s.DurationMS, &s.Kind, &s.Name, &s.Model, &s.Provider,
			&s.InputTokens, &s.OutputTokens, &s.CostUSD,
			&s.ToolName, &s.ToolArgsRedacted, &s.ToolResultRedacted,
			&s.PolicyDecision, &s.PolicyID, &s.PolicyReason,
			&s.GuardrailHits, &s.Attributes,
		); err != nil {
			return nil, err
		}
		s.TS = ts.UTC().Format(time.RFC3339Nano)
		out = append(out, s)
	}
	return out, rows.Err()
}

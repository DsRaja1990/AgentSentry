-- AgentSentry ClickHouse schema v0.1
-- DB `sentry` is created by docker (CLICKHOUSE_DB env var).
create table if not exists agent_span (
    trace_id              String,
    span_id               String,
    parent_id             String,

    tenant_id             LowCardinality(String),
    project_id            LowCardinality(String),
    agent_id              LowCardinality(String),

    ts                    DateTime64(3, 'UTC'),
    duration_ms           UInt64,

    kind                  LowCardinality(String),
    name                  String,
    model                 LowCardinality(String),
    provider              LowCardinality(String),

    input_tokens          UInt64,
    output_tokens         UInt64,
    cost_usd              Float64,

    tool_name             String,
    tool_args_redacted    String,
    tool_result_redacted  String,

    policy_decision       LowCardinality(String),
    policy_id             LowCardinality(String),
    policy_reason         String,

    guardrail_hits        Array(String),
    attributes            Map(String, String)
) engine = MergeTree
  partition by (tenant_id, toYYYYMM(ts))
  order by (tenant_id, agent_id, ts)
  ttl toDateTime(ts) + interval 90 day

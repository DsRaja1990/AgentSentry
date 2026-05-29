package main

import (
	"context"

	"github.com/agentsentry/agentsentry/control/internal/model"
	"github.com/agentsentry/agentsentry/control/internal/store"
)

// seedDemo inserts a default project, sample agent, the dev API key, and a
// canonical "block-external-email" Rego policy if absent.
func seedDemo(ctx context.Context, pg *store.Postgres, devKey string) error {
	// Default tenant + project (FK order matters).
	if _, err := pg.Pool.Exec(ctx,
		`insert into tenant (id, name, plan) values ('t_default','Default Tenant','dev')
		 on conflict (id) do nothing`); err != nil {
		return err
	}
	if _, err := pg.Pool.Exec(ctx,
		`insert into project (id, tenant_id, name) values ('p_default','t_default','Default')
		 on conflict (id) do nothing`); err != nil {
		return err
	}

	// Demo agent.
	_, _ = pg.CreateAgent(ctx, model.Agent{
		ID: "agt_support_bot", ProjectID: "p_default",
		Name: "support-bot", Framework: "langchain",
		Owner: "demo", IdentityKind: "api_key", IdentityRef: "dev",
	})

	// Demo policy: block external email recipients via tool call.
	_, _ = pg.UpsertPolicy(ctx, model.Policy{
		ID: "pol_block_external_email", ProjectID: "p_default",
		Name: "block-external-email", Language: "rego",
		Status: "enforced",
		Source: `package agentsentry.tool_call

import future.keywords.if
import future.keywords.in

default decision := {"allow": true}

decision := {
    "allow": false,
    "reason": "external email recipient",
    "policy_id": "pol_block_external_email",
    "obligations": ["log_to_audit"]
} if {
    input.tool.name == "send_email"
    not endswith(input.tool.args.to, "@contoso.com")
}`,
	})

	// Demo policy: block prompts mentioning prod credentials.
	_, _ = pg.UpsertPolicy(ctx, model.Policy{
		ID: "pol_block_prod_creds", ProjectID: "p_default",
		Name: "block-prod-credentials", Language: "rego",
		Status: "enforced",
		Source: `package agentsentry.llm_call

import future.keywords.if
import future.keywords.in

default decision := {"allow": true}

decision := {
    "allow": false,
    "reason": "prompt contains production credential reference",
    "policy_id": "pol_block_prod_creds"
} if {
    some msg in input.request.body.messages
    contains(lower(msg.content), "prod_db_password")
}`,
	})

	// Dev API key — explicit insert so its hash matches.
	if devKey != "" {
		hashed := store.HashKey(devKey)
		if _, err := pg.Pool.Exec(ctx,
			`insert into api_key (id, tenant_id, hashed_key, scopes)
			 values ('key_dev_seed', 't_default', $1, ARRAY['ingest','policy_check','admin'])
			 on conflict (id) do nothing`, hashed); err != nil {
			return err
		}
	}
	return nil
}

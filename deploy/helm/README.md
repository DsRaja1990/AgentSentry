# Helm chart — Phase 2 stub

A production Helm chart for AgentSentry will live here. Planned layout:

```
deploy/helm/agentsentry/
├── Chart.yaml
├── values.yaml
└── templates/
    ├── gateway-deploy.yaml
    ├── gateway-service.yaml
    ├── control-deploy.yaml
    ├── control-service.yaml
    ├── ui-deploy.yaml
    ├── ui-service.yaml
    ├── postgres.yaml          # subchart or external
    ├── clickhouse.yaml        # subchart or external
    └── networkpolicies.yaml
```

For now, use `deploy/docker-compose.dev.yml` for local development and
adapt to your platform of choice.

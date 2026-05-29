# AgentSentry UI (Next.js 15)

Server-rendered dashboard. Server components fetch from the control plane via
`CONTROL_API_URL` (default `http://control:8081`) using the dev API key.

## Pages

- `/`           Dashboard summary
- `/traces`     Recent spans (filterable)
- `/agents`     Registered agents
- `/policies`   Policy list
- `/policies/[id]` Policy source viewer

## Dev

```powershell
cd ui
npm install
$env:CONTROL_API_URL = "http://localhost:8081"
$env:CONTROL_API_KEY = "sk_dev_local_demo_key"
npm run dev
```

## Phase 2

Policy editor (Monaco), live trace tail (SSE), audit-event viewer with
hash-chain verification, RBAC + SSO, multi-tenant tenant switcher.

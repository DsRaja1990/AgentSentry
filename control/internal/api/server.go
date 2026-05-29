package api

import (
	"context"
	"encoding/json"
	"errors"
	"log/slog"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/agentsentry/agentsentry/control/internal/model"
	"github.com/agentsentry/agentsentry/control/internal/store"
	"github.com/go-chi/chi/v5"
)

type Server struct {
	PG  *store.Postgres
	CH  *store.Clickhouse
	Log *slog.Logger
	// DevKey is a fixed dev API key accepted when set (non-empty).
	DevKey string
}

func (s *Server) Routes() http.Handler {
	r := chi.NewRouter()
	r.Use(loggingMiddleware(s.Log))
	r.Use(corsMiddleware)

	r.Get("/v1/health", s.health)

	// Auth-gated APIs.
	r.Group(func(g chi.Router) {
		g.Use(s.authMiddleware)

		g.Post("/v1/ingest",         s.ingestSpans)

		g.Get( "/v1/agents",         s.listAgents)
		g.Post("/v1/agents",         s.createAgent)

		g.Get( "/v1/policies",       s.listPolicies)
		g.Post("/v1/policies",       s.upsertPolicy)
		g.Put( "/v1/policies/{id}",  s.upsertPolicyByID)
		g.Get( "/v1/policies/{id}",  s.getPolicy)
		g.Get( "/v1/policies/bundle",s.policyBundle)

		g.Get( "/v1/traces",         s.queryTraces)

		g.Post("/v1/api-keys",       s.createAPIKey)
	})

	return r
}

// ---------------------------------------------------------------- middleware

func loggingMiddleware(log *slog.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()
			rw := &statusRecorder{ResponseWriter: w, status: 200}
			next.ServeHTTP(rw, r)
			log.Info("http",
				"method", r.Method, "path", r.URL.Path,
				"status", rw.status, "dur_ms", time.Since(start).Milliseconds())
		})
	}
}

type statusRecorder struct {
	http.ResponseWriter
	status int
}
func (s *statusRecorder) WriteHeader(code int) { s.status = code; s.ResponseWriter.WriteHeader(code) }

func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin",  "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET,POST,PUT,DELETE,OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Authorization, Content-Type")
		if r.Method == http.MethodOptions { w.WriteHeader(http.StatusNoContent); return }
		next.ServeHTTP(w, r)
	})
}

type ctxKey string
const ctxTenant ctxKey = "tenant_id"

func (s *Server) authMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		raw := bearer(r)
		if raw == "" {
			httpErr(w, http.StatusUnauthorized, "missing bearer token"); return
		}
		// Dev key shortcut.
		if s.DevKey != "" && raw == s.DevKey {
			r = r.WithContext(context.WithValue(r.Context(), ctxTenant, "t_default"))
			next.ServeHTTP(w, r); return
		}
		k, err := s.PG.LookupAPIKey(r.Context(), raw)
		if err != nil {
			if errors.Is(err, store.ErrNotFound) {
				httpErr(w, http.StatusUnauthorized, "invalid token"); return
			}
			httpErr(w, http.StatusInternalServerError, "auth error"); return
		}
		if k.RevokedAt != nil {
			httpErr(w, http.StatusUnauthorized, "revoked"); return
		}
		r = r.WithContext(context.WithValue(r.Context(), ctxTenant, k.TenantID))
		next.ServeHTTP(w, r)
	})
}

func bearer(r *http.Request) string {
	h := r.Header.Get("Authorization")
	if strings.HasPrefix(strings.ToLower(h), "bearer ") { return strings.TrimSpace(h[7:]) }
	if v := r.Header.Get("x-agentsentry-key"); v != "" { return v }
	return ""
}

// ---------------------------------------------------------------- handlers

func (s *Server) health(w http.ResponseWriter, _ *http.Request) {
	writeJSON(w, 200, map[string]string{"status": "ok"})
}

func (s *Server) ingestSpans(w http.ResponseWriter, r *http.Request) {
	var req model.IngestRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		httpErr(w, 400, "bad json"); return
	}
	// Resolve tenant per-span from caller_key_hash. Cache lookups inside the
	// request to keep batch ingest cheap. Spans without a hash inherit the
	// auth-context tenant (the gateway's own key, dev key, etc.).
	cache := map[string]string{} // hash -> tenant_id
	fallbackTenant := tenantOf(r)
	for i := range req.Spans {
		h := req.Spans[i].CallerKeyHash
		if h == "" {
			if req.Spans[i].TenantID == "" { req.Spans[i].TenantID = fallbackTenant }
			continue
		}
		if t, ok := cache[h]; ok {
			req.Spans[i].TenantID = t
		} else {
			k, err := s.PG.LookupAPIKeyByHash(r.Context(), h)
			if err == nil {
				cache[h] = k.TenantID
				req.Spans[i].TenantID = k.TenantID
			} else {
				cache[h] = fallbackTenant
				req.Spans[i].TenantID = fallbackTenant
			}
		}
		req.Spans[i].CallerKeyHash = "" // do not persist
	}
	if err := s.CH.InsertSpans(r.Context(), req.Spans); err != nil {
		s.Log.Warn("insert spans", "err", err)
		httpErr(w, 500, "ingest failed"); return
	}
	writeJSON(w, 200, model.IngestResponse{Accepted: uint64(len(req.Spans))})
}

func (s *Server) listAgents(w http.ResponseWriter, r *http.Request) {
	a, err := s.PG.ListAgents(r.Context())
	if err != nil { httpErr(w, 500, err.Error()); return }
	writeJSON(w, 200, map[string]any{"items": a})
}

func (s *Server) createAgent(w http.ResponseWriter, r *http.Request) {
	var a model.Agent
	if err := json.NewDecoder(r.Body).Decode(&a); err != nil {
		httpErr(w, 400, "bad json"); return
	}
	out, err := s.PG.CreateAgent(r.Context(), a)
	if err != nil { httpErr(w, 500, err.Error()); return }
	_ = s.PG.AppendAudit(r.Context(), tenantOf(r), "api", "agent.create", out.ID, jsonBytes(out))
	writeJSON(w, 201, out)
}

func (s *Server) listPolicies(w http.ResponseWriter, r *http.Request) {
	p, err := s.PG.ListPolicies(r.Context())
	if err != nil { httpErr(w, 500, err.Error()); return }
	writeJSON(w, 200, map[string]any{"items": p})
}

func (s *Server) getPolicy(w http.ResponseWriter, r *http.Request) {
	id := chi.URLParam(r, "id")
	p, err := s.PG.GetPolicy(r.Context(), id)
	if errors.Is(err, store.ErrNotFound) { httpErr(w, 404, "not found"); return }
	if err != nil                        { httpErr(w, 500, err.Error()); return }
	writeJSON(w, 200, p)
}

func (s *Server) upsertPolicy(w http.ResponseWriter, r *http.Request) {
	var p model.Policy
	if err := json.NewDecoder(r.Body).Decode(&p); err != nil {
		httpErr(w, 400, "bad json"); return
	}
	out, err := s.PG.UpsertPolicy(r.Context(), p)
	if err != nil { httpErr(w, 500, err.Error()); return }
	_ = s.PG.AppendAudit(r.Context(), tenantOf(r), "api", "policy.upsert", out.ID, jsonBytes(out))
	writeJSON(w, 200, out)
}

func (s *Server) upsertPolicyByID(w http.ResponseWriter, r *http.Request) {
	var p model.Policy
	if err := json.NewDecoder(r.Body).Decode(&p); err != nil {
		httpErr(w, 400, "bad json"); return
	}
	p.ID = chi.URLParam(r, "id")
	out, err := s.PG.UpsertPolicy(r.Context(), p)
	if err != nil { httpErr(w, 500, err.Error()); return }
	_ = s.PG.AppendAudit(r.Context(), tenantOf(r), "api", "policy.upsert", out.ID, jsonBytes(out))
	writeJSON(w, 200, out)
}

func (s *Server) policyBundle(w http.ResponseWriter, r *http.Request) {
	all, err := s.PG.ListPolicies(r.Context())
	if err != nil { httpErr(w, 500, err.Error()); return }
	enforced := all[:0]
	for _, p := range all {
		if p.Status == "enforced" || p.Status == "monitor" {
			enforced = append(enforced, p)
		}
	}
	writeJSON(w, 200, model.PolicyBundle{Version: uint64(time.Now().Unix()), Policies: enforced})
}

func (s *Server) queryTraces(w http.ResponseWriter, r *http.Request) {
	q := r.URL.Query()
	f := store.TraceFilter{
		AgentID:  q.Get("agent_id"),
		Decision: q.Get("decision"),
	}
	if v := q.Get("limit"); v != "" {
		if n, err := strconv.Atoi(v); err == nil { f.Limit = n }
	}
	if v := q.Get("since"); v != "" {
		if t, err := time.Parse(time.RFC3339, v); err == nil { f.Since = &t }
	}
	if v := q.Get("until"); v != "" {
		if t, err := time.Parse(time.RFC3339, v); err == nil { f.Until = &t }
	}
	out, err := s.CH.QueryTraces(r.Context(), f)
	if err != nil { httpErr(w, 500, err.Error()); return }
	writeJSON(w, 200, map[string]any{"items": out})
}

func (s *Server) createAPIKey(w http.ResponseWriter, r *http.Request) {
	var k model.APIKey
	if err := json.NewDecoder(r.Body).Decode(&k); err != nil {
		httpErr(w, 400, "bad json"); return
	}
	out, err := s.PG.CreateAPIKey(r.Context(), k)
	if err != nil { httpErr(w, 500, err.Error()); return }
	_ = s.PG.AppendAudit(r.Context(), tenantOf(r), "api", "apikey.create", out.ID, jsonBytes(map[string]any{"id": out.ID, "scopes": out.Scopes}))
	writeJSON(w, 201, out)
}

// ---------------------------------------------------------------- helpers

func writeJSON(w http.ResponseWriter, code int, v any) {
	w.Header().Set("content-type", "application/json")
	w.WriteHeader(code)
	_ = json.NewEncoder(w).Encode(v)
}
func httpErr(w http.ResponseWriter, code int, msg string) {
	writeJSON(w, code, map[string]string{"error": msg})
}
func jsonBytes(v any) []byte { b, _ := json.Marshal(v); return b }
func tenantOf(r *http.Request) string {
	if v, ok := r.Context().Value(ctxTenant).(string); ok { return v }
	return "t_default"
}

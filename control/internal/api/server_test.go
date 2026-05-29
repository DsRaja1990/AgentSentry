package api

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func TestBearerExtraction(t *testing.T) {
	cases := []struct{ hdr, val, want string }{
		{"Authorization",     "Bearer abc",       "abc"},
		{"authorization",     "bearer  spaced  ", "spaced"},
		{"x-agentsentry-key", "raw_key",          "raw_key"},
	}
	for _, c := range cases {
		r := httptest.NewRequest("GET", "/", nil)
		r.Header.Set(c.hdr, c.val)
		got := bearer(r)
		if got != c.want { t.Errorf("hdr=%s val=%q -> %q, want %q", c.hdr, c.val, got, c.want) }
	}
}

func TestAuthMiddlewareRejectsMissingToken(t *testing.T) {
	s := &Server{DevKey: "sk_dev"}
	called := false
	h := s.authMiddleware(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) { called = true }))
	rr := httptest.NewRecorder()
	h.ServeHTTP(rr, httptest.NewRequest("GET", "/", nil))
	if called          { t.Fatal("handler must not run") }
	if rr.Code != 401  { t.Fatalf("want 401, got %d", rr.Code) }
}

func TestAuthMiddlewareAcceptsDevKey(t *testing.T) {
	s := &Server{DevKey: "sk_dev"}
	rr := httptest.NewRecorder()
	r := httptest.NewRequest("GET", "/", nil)
	r.Header.Set("Authorization", "Bearer sk_dev")
	gotTenant := ""
	h := s.authMiddleware(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		gotTenant = tenantOf(r)
	}))
	h.ServeHTTP(rr, r)
	if rr.Code != 200            { t.Fatalf("want 200, got %d", rr.Code) }
	if gotTenant != "t_default"  { t.Fatalf("want t_default, got %q", gotTenant) }
}

func TestCORSPreflight(t *testing.T) {
	rr := httptest.NewRecorder()
	r := httptest.NewRequest("OPTIONS", "/", nil)
	corsMiddleware(http.HandlerFunc(func(http.ResponseWriter, *http.Request) {})).ServeHTTP(rr, r)
	if rr.Code != 204 { t.Fatalf("want 204, got %d", rr.Code) }
	if !strings.Contains(rr.Header().Get("Access-Control-Allow-Methods"), "POST") {
		t.Fatal("missing POST in allow-methods")
	}
}

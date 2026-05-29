package main

import (
	"context"
	"flag"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"syscall"
	"time"

	"github.com/agentsentry/agentsentry/control/internal/api"
	"github.com/agentsentry/agentsentry/control/internal/store"

	yaml "gopkg.in/yaml.v3"
)

type Config struct {
	ListenAddr     string `yaml:"listen_addr"`
	PostgresDSN    string `yaml:"postgres_dsn"`
	ClickhouseDSN  string `yaml:"clickhouse_dsn"`
	DevAPIKey      string `yaml:"dev_api_key"`
	SeedDemoData   bool   `yaml:"seed_demo_data"`
}

func main() {
	cfgPath := flag.String("config", "config.yaml", "path to YAML config")
	flag.Parse()

	cfg := Config{
		ListenAddr:    ":8081",
		PostgresDSN:   envOr("POSTGRES_DSN",   "postgres://sentry:sentry@postgres:5432/sentry?sslmode=disable"),
		ClickhouseDSN: envOr("CLICKHOUSE_DSN", "clickhouse://default:@clickhouse:9000/sentry"),
		DevAPIKey:     envOr("SENTRY_DEV_API_KEY", "sk_dev_local_demo_key"),
		SeedDemoData:  envOr("SENTRY_SEED_DEMO", "true") == "true",
	}
	if data, err := os.ReadFile(*cfgPath); err == nil {
		_ = yaml.Unmarshal(data, &cfg)
	}

	log := slog.New(slog.NewJSONHandler(os.Stdout, &slog.HandlerOptions{Level: slog.LevelInfo}))
	slog.SetDefault(log)

	ctx, cancel := signal.NotifyContext(context.Background(), os.Interrupt, syscall.SIGTERM)
	defer cancel()

	pg, err := waitPostgres(ctx, cfg.PostgresDSN, log)
	if err != nil { fatal(log, "postgres", err) }

	if err := runMigrations(ctx, pg, "migrations/postgres"); err != nil {
		fatal(log, "pg migrations", err)
	}
	if cfg.SeedDemoData {
		if err := seedDemo(ctx, pg, cfg.DevAPIKey); err != nil {
			log.Warn("seed demo", "err", err)
		}
	}

	ch, err := waitClickhouse(ctx, cfg.ClickhouseDSN, log)
	if err != nil { fatal(log, "clickhouse", err) }

	if err := runMigrationsCH(ctx, ch, "migrations/clickhouse"); err != nil {
		fatal(log, "ch migrations", err)
	}

	srv := &api.Server{PG: pg, CH: ch, Log: log, DevKey: cfg.DevAPIKey}
	httpSrv := &http.Server{
		Addr:              cfg.ListenAddr,
		Handler:           srv.Routes(),
		ReadHeaderTimeout: 5 * time.Second,
	}
	go func() {
		log.Info("control plane listening", "addr", cfg.ListenAddr)
		if err := httpSrv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			fatal(log, "http listen", err)
		}
	}()

	<-ctx.Done()
	shutCtx, cancelShut := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancelShut()
	_ = httpSrv.Shutdown(shutCtx)
}

func envOr(name, def string) string { if v := os.Getenv(name); v != "" { return v }; return def }

func fatal(log *slog.Logger, where string, err error) {
	log.Error("fatal", "where", where, "err", err); os.Exit(1)
}

func waitPostgres(ctx context.Context, dsn string, log *slog.Logger) (*store.Postgres, error) {
	for i := 0; i < 30; i++ {
		p, err := store.NewPostgres(ctx, dsn)
		if err == nil { return p, nil }
		log.Info("waiting for postgres", "attempt", i+1, "err", err.Error())
		select {
		case <-ctx.Done(): return nil, ctx.Err()
		case <-time.After(2 * time.Second):
		}
	}
	return nil, fmt.Errorf("postgres not ready after 60s")
}

func waitClickhouse(ctx context.Context, dsn string, log *slog.Logger) (*store.Clickhouse, error) {
	for i := 0; i < 30; i++ {
		c, err := store.NewClickhouse(ctx, dsn)
		if err == nil { return c, nil }
		log.Info("waiting for clickhouse", "attempt", i+1, "err", err.Error())
		select {
		case <-ctx.Done(): return nil, ctx.Err()
		case <-time.After(2 * time.Second):
		}
	}
	return nil, fmt.Errorf("clickhouse not ready after 60s")
}

// ---------------------------------------------------------------- migrations

func runMigrations(ctx context.Context, pg *store.Postgres, dir string) error {
	files, err := readSorted(dir, ".sql")
	if err != nil { return err }
	for _, f := range files {
		sql, err := os.ReadFile(f)
		if err != nil { return err }
		for _, stmt := range splitSQL(string(sql)) {
			s := strings.TrimSpace(stmt)
			if s == "" || strings.HasPrefix(s, "--") { continue }
			if _, err := pg.Pool.Exec(ctx, s); err != nil {
				return fmt.Errorf("%s: %w\n--stmt--\n%s", f, err, s)
			}
		}
	}
	return nil
}

func runMigrationsCH(ctx context.Context, ch *store.Clickhouse, dir string) error {
	files, err := readSorted(dir, ".sql")
	if err != nil { return err }
	for _, f := range files {
		sql, err := os.ReadFile(f)
		if err != nil { return err }
		for _, stmt := range splitSQL(string(sql)) {
			if strings.TrimSpace(stmt) == "" { continue }
			if err := ch.Conn.Exec(ctx, stmt); err != nil {
				return fmt.Errorf("%s: %w", f, err)
			}
		}
	}
	return nil
}

func readSorted(dir, ext string) ([]string, error) {
	ents, err := os.ReadDir(dir)
	if err != nil { return nil, nil } // missing dir => no migrations
	var out []string
	for _, e := range ents {
		if !e.IsDir() && strings.HasSuffix(e.Name(), ext) {
			out = append(out, dir+"/"+e.Name())
		}
	}
	return out, nil
}

func splitSQL(s string) []string { return strings.Split(s, ";\n") }

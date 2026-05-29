package store

import "testing"

func TestHashKeyDeterministic(t *testing.T) {
	a := HashKey("sk_test")
	b := HashKey("sk_test")
	if a != b           { t.Fatalf("hash mismatch %s != %s", a, b) }
	if len(a) != 64     { t.Fatalf("expected 64-char hex, got %d", len(a)) }
	if HashKey("a") == HashKey("b") { t.Fatal("collision") }
}

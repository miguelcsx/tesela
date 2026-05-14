package ratelimit

import "testing"

func TestMemoryLimiter_AllowsBurst(t *testing.T) {
	l := NewMemory(1, 3)
	for i := 0; i < 3; i++ {
		if !l.Allow("ws", "ip") {
			t.Fatalf("expected allow %d", i)
		}
	}
	if l.Allow("ws", "ip") {
		t.Fatal("expected deny after burst exhausted")
	}
}

func TestMemoryLimiter_PerKeyIsolation(t *testing.T) {
	l := NewMemory(1, 1)
	if !l.Allow("ws", "a") {
		t.Fatal()
	}
	if !l.Allow("ws", "b") {
		t.Fatal("expected allow for distinct key")
	}
}

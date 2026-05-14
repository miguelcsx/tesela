package crypto_test

import (
	"bytes"
	"strings"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/crypto"
)

const (
	testKey  = "0123456789abcdef0123456789abcdef" // 32 bytes
	otherKey = "fedcba9876543210fedcba9876543210"
	shortKey = "shortkey"
)

func TestSealer_RoundTrip(t *testing.T) {
	t.Parallel()

	s, err := crypto.NewAESGCMSealer([]byte(testKey))
	if err != nil {
		t.Fatalf("NewAESGCMSealer: %v", err)
	}

	plaintext := []byte("postgres://user:hunter2@host:5432/db")
	sealed, err := s.Seal(plaintext)
	if err != nil {
		t.Fatalf("Seal: %v", err)
	}
	if bytes.Equal(sealed, plaintext) {
		t.Fatal("sealed must differ from plaintext")
	}

	got, err := s.Open(sealed)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	if !bytes.Equal(got, plaintext) {
		t.Fatalf("Open returned %q, want %q", got, plaintext)
	}
}

func TestSealer_DistinctNoncesProduceDistinctCiphertexts(t *testing.T) {
	t.Parallel()

	s, err := crypto.NewAESGCMSealer([]byte(testKey))
	if err != nil {
		t.Fatalf("NewAESGCMSealer: %v", err)
	}

	plaintext := []byte("same input every time")
	a, _ := s.Seal(plaintext)
	b, _ := s.Seal(plaintext)
	if bytes.Equal(a, b) {
		t.Fatal("two seals of the same plaintext must differ (random nonce)")
	}
}

func TestSealer_TamperDetected(t *testing.T) {
	t.Parallel()

	s, err := crypto.NewAESGCMSealer([]byte(testKey))
	if err != nil {
		t.Fatalf("NewAESGCMSealer: %v", err)
	}

	sealed, err := s.Seal([]byte("payload"))
	if err != nil {
		t.Fatalf("Seal: %v", err)
	}
	// Flip a bit in the ciphertext portion (after the nonce).
	tampered := make([]byte, len(sealed))
	copy(tampered, sealed)
	tampered[len(tampered)-1] ^= 0x01
	if _, err := s.Open(tampered); err == nil {
		t.Fatal("Open must fail on tampered ciphertext")
	}
}

func TestSealer_WrongKeyFailsOpen(t *testing.T) {
	t.Parallel()

	enc, err := crypto.NewAESGCMSealer([]byte(testKey))
	if err != nil {
		t.Fatalf("NewAESGCMSealer: %v", err)
	}
	dec, err := crypto.NewAESGCMSealer([]byte(otherKey))
	if err != nil {
		t.Fatalf("NewAESGCMSealer: %v", err)
	}
	sealed, _ := enc.Seal([]byte("payload"))
	if _, err := dec.Open(sealed); err == nil {
		t.Fatal("Open with different key must fail")
	}
}

func TestSealer_RejectsShortKey(t *testing.T) {
	t.Parallel()

	if _, err := crypto.NewAESGCMSealer([]byte(shortKey)); err == nil {
		t.Fatal("NewAESGCMSealer must reject short keys")
	}
}

func TestSealer_RejectsTooShortCiphertext(t *testing.T) {
	t.Parallel()

	s, _ := crypto.NewAESGCMSealer([]byte(testKey))
	if _, err := s.Open([]byte("too-short")); err == nil {
		t.Fatal("Open must reject ciphertext shorter than the nonce")
	}
}

func TestSealer_AcceptsAlternateValidKeyLengths(t *testing.T) {
	t.Parallel()

	// AES-128 (16-byte key) and AES-192 (24-byte key) must also work.
	for _, k := range []string{
		"abcdefghijklmnop",         // 16
		"abcdefghijklmnopqrstuvwx", // 24
	} {
		s, err := crypto.NewAESGCMSealer([]byte(k))
		if err != nil {
			t.Fatalf("NewAESGCMSealer(%d-byte key): %v", len(k), err)
		}
		sealed, err := s.Seal([]byte("hi"))
		if err != nil {
			t.Fatalf("Seal: %v", err)
		}
		if got, _ := s.Open(sealed); string(got) != "hi" {
			t.Fatalf("round-trip with %d-byte key failed", len(k))
		}
	}
}

func TestSealer_NoncePrefixedToCiphertext(t *testing.T) {
	t.Parallel()

	s, _ := crypto.NewAESGCMSealer([]byte(testKey))
	sealed, _ := s.Seal([]byte("p"))
	if len(sealed) < crypto.NonceSize {
		t.Fatalf("sealed is shorter than NonceSize: %d", len(sealed))
	}
}

func TestSealer_EmptyPlaintextRoundTrips(t *testing.T) {
	t.Parallel()

	s, _ := crypto.NewAESGCMSealer([]byte(testKey))
	sealed, err := s.Seal(nil)
	if err != nil {
		t.Fatalf("Seal(nil): %v", err)
	}
	got, err := s.Open(sealed)
	if err != nil {
		t.Fatalf("Open: %v", err)
	}
	if len(got) != 0 {
		t.Fatalf("expected empty plaintext, got %q", got)
	}
}

func TestKeyFromHex_RoundTrip(t *testing.T) {
	t.Parallel()

	hex := strings.Repeat("ab", 32) // 32 bytes hex-encoded → 32 raw bytes? no, 64 hex chars → 32 bytes.
	key, err := crypto.KeyFromHex(hex)
	if err != nil {
		t.Fatalf("KeyFromHex: %v", err)
	}
	if len(key) != 32 {
		t.Fatalf("expected 32-byte key, got %d", len(key))
	}
}

func TestKeyFromHex_RejectsBadLength(t *testing.T) {
	t.Parallel()

	if _, err := crypto.KeyFromHex("ab"); err == nil {
		t.Fatal("KeyFromHex must reject invalid lengths")
	}
}

func TestKeyFromHex_RejectsBadHex(t *testing.T) {
	t.Parallel()

	if _, err := crypto.KeyFromHex(strings.Repeat("zz", 16)); err == nil {
		t.Fatal("KeyFromHex must reject invalid hex")
	}
}

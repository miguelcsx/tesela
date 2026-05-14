package secrets_test

import (
	"context"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/secrets"
)

func TestEnvProvider_LookupReturnsValue(t *testing.T) {
	t.Setenv("LATTICE_TEST_SECRET", "supersecret")

	p := secrets.NewEnvProvider()
	got, err := p.Lookup(context.Background(), "LATTICE_TEST_SECRET")
	if err != nil {
		t.Fatalf("Lookup: %v", err)
	}
	if got != "supersecret" {
		t.Fatalf("got %q, want %q", got, "supersecret")
	}
}

func TestEnvProvider_LookupMissingReturnsErrNotFound(t *testing.T) {
	p := secrets.NewEnvProvider()
	_, err := p.Lookup(context.Background(), "LATTICE_TEST_DOES_NOT_EXIST")
	if err == nil {
		t.Fatal("expected error, got nil")
	}
	if !secrets.IsNotFound(err) {
		t.Fatalf("expected IsNotFound, got %v", err)
	}
}

func TestEnvProvider_RejectsEmptyReference(t *testing.T) {
	p := secrets.NewEnvProvider()
	_, err := p.Lookup(context.Background(), "")
	if err == nil {
		t.Fatal("expected error for empty reference")
	}
}

func TestEnvProvider_Name(t *testing.T) {
	p := secrets.NewEnvProvider()
	if p.Name() != "env" {
		t.Fatalf("Name = %q, want env", p.Name())
	}
}

func TestStaticProvider_Lookup(t *testing.T) {
	p := secrets.NewStaticProvider(map[string]string{
		"db.password":  "abc",
		"webhook.salt": "xyz",
	})
	got, err := p.Lookup(context.Background(), "db.password")
	if err != nil {
		t.Fatalf("Lookup: %v", err)
	}
	if got != "abc" {
		t.Fatalf("got %q, want abc", got)
	}
	if _, err := p.Lookup(context.Background(), "missing"); !secrets.IsNotFound(err) {
		t.Fatalf("expected IsNotFound, got %v", err)
	}
}

func TestStaticProvider_Name(t *testing.T) {
	p := secrets.NewStaticProvider(nil)
	if p.Name() != "static" {
		t.Fatalf("Name = %q, want static", p.Name())
	}
}

func TestResolveReference_LiteralString(t *testing.T) {
	p := secrets.NewStaticProvider(map[string]string{"k": "v"})
	got, err := secrets.ResolveReference(context.Background(), p, "literal-value")
	if err != nil {
		t.Fatalf("ResolveReference: %v", err)
	}
	if got != "literal-value" {
		t.Fatalf("got %q, want literal-value", got)
	}
}

func TestResolveReference_DollarBraceLookup(t *testing.T) {
	p := secrets.NewStaticProvider(map[string]string{"DB_PASS": "shh"})
	got, err := secrets.ResolveReference(context.Background(), p, "${DB_PASS}")
	if err != nil {
		t.Fatalf("ResolveReference: %v", err)
	}
	if got != "shh" {
		t.Fatalf("got %q, want shh", got)
	}
}

func TestResolveReference_MissingReferenceErrors(t *testing.T) {
	p := secrets.NewStaticProvider(nil)
	if _, err := secrets.ResolveReference(context.Background(), p, "${NOPE}"); !secrets.IsNotFound(err) {
		t.Fatalf("expected IsNotFound, got %v", err)
	}
}

func TestResolveReference_DollarSignWithoutBraceIsLiteral(t *testing.T) {
	p := secrets.NewStaticProvider(nil)
	got, err := secrets.ResolveReference(context.Background(), p, "$NOTREF")
	if err != nil {
		t.Fatalf("ResolveReference: %v", err)
	}
	if got != "$NOTREF" {
		t.Fatalf("got %q, want $NOTREF (literal)", got)
	}
}

func TestResolveReferences_Map(t *testing.T) {
	p := secrets.NewStaticProvider(map[string]string{"PWD": "abc"})
	in := map[string]string{
		"username": "user",
		"password": "${PWD}",
	}
	out, err := secrets.ResolveReferences(context.Background(), p, in)
	if err != nil {
		t.Fatalf("ResolveReferences: %v", err)
	}
	if out["password"] != "abc" || out["username"] != "user" {
		t.Fatalf("ResolveReferences result: %+v", out)
	}
}

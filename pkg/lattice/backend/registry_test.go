package backend

import (
	"context"
	"errors"
	"testing"

	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

type fakeAdapter struct {
	connectErr error
	connect    func(types.ConfigMap) (Connection, error)
}

func (a *fakeAdapter) Type() string { return "fake" }
func (a *fakeAdapter) Connect(_ context.Context, cfg types.ConfigMap) (Connection, error) {
	if a.connectErr != nil {
		return nil, a.connectErr
	}
	if a.connect != nil {
		return a.connect(cfg)
	}
	return &fakeConn{cfg: cfg}, nil
}

type fakeConn struct {
	cfg    types.ConfigMap
	closed bool
}

func (c *fakeConn) Get(context.Context, types.SourceConfig, types.ObjectType, any, types.Filter) (types.Record, error) {
	return types.Record{}, nil
}

func (c *fakeConn) Search(context.Context, types.SourceConfig, types.ObjectType, types.QuerySpec, types.Filter) (types.Page, error) {
	return types.Page{}, nil
}

func (c *fakeConn) Aggregate(context.Context, types.SourceConfig, types.ObjectType, types.AggregateSpec, types.Filter) (types.AggregateResult, error) {
	return types.AggregateResult{}, nil
}

func (c *fakeConn) Traverse(context.Context, types.SourceConfig, types.LinkType, types.ObjectType, []any, types.QuerySpec, types.Filter) (types.Page, error) {
	return types.Page{}, nil
}

func (c *fakeConn) Mutate(context.Context, types.SourceConfig, types.Mutation) (types.MutationResult, error) {
	return types.MutationResult{}, nil
}

func (c *fakeConn) Ping(context.Context) error    { return nil }
func (c *fakeConn) Close(_ context.Context) error { c.closed = true; return nil }

func TestRegistry_RegisterAndDriver(t *testing.T) {
	r := NewRegistry(nil)
	a := &fakeAdapter{}
	r.Register(a)

	got, ok := r.Driver("fake")
	if !ok || got.Type() != "fake" {
		t.Fatalf("expected fake driver, got %v ok=%v", got, ok)
	}

	if _, ok := r.Driver("missing"); ok {
		t.Fatal("expected missing driver to be absent")
	}
}

func TestRegistry_AcquireCachesConnection(t *testing.T) {
	r := NewRegistry(nil)
	r.Register(&fakeAdapter{})

	ds := types.Datasource{
		WorkspaceID: "ws-1",
		APIName:     "primary",
		AdapterType: "fake",
	}
	c1, err := r.Acquire(context.Background(), ds)
	if err != nil {
		t.Fatalf("acquire: %v", err)
	}
	c2, err := r.Acquire(context.Background(), ds)
	if err != nil {
		t.Fatalf("acquire2: %v", err)
	}
	if c1 != c2 {
		t.Fatal("expected cached connection to be returned again")
	}
}

func TestRegistry_AcquireUnknownAdapter(t *testing.T) {
	r := NewRegistry(nil)
	_, err := r.Acquire(context.Background(), types.Datasource{AdapterType: "nope"})
	if err == nil {
		t.Fatal("expected error for unknown adapter")
	}
}

func TestRegistry_EvictClosesConnection(t *testing.T) {
	r := NewRegistry(nil)
	r.Register(&fakeAdapter{})
	ds := types.Datasource{WorkspaceID: "ws-1", APIName: "primary", AdapterType: "fake"}
	c, err := r.Acquire(context.Background(), ds)
	if err != nil {
		t.Fatalf("acquire: %v", err)
	}
	if err := r.Evict(context.Background(), ds.WorkspaceID, ds.APIName); err != nil {
		t.Fatalf("evict: %v", err)
	}
	if !c.(*fakeConn).closed {
		t.Fatal("expected eviction to close the connection")
	}
}

func TestRegistry_AcquireConnectError(t *testing.T) {
	r := NewRegistry(nil)
	r.Register(&fakeAdapter{connectErr: errors.New("boom")})
	_, err := r.Acquire(context.Background(), types.Datasource{AdapterType: "fake"})
	if err == nil {
		t.Fatal("expected connect error to propagate")
	}
}

func TestRegistry_SealedCredentialsRequireSealer(t *testing.T) {
	r := NewRegistry(nil)
	r.Register(&fakeAdapter{})
	_, err := r.Acquire(context.Background(), types.Datasource{
		AdapterType:       "fake",
		SealedCredentials: []byte{0x01, 0x02},
	})
	if err == nil {
		t.Fatal("expected sealed credentials without sealer to error")
	}
}

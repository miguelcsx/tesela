package events

import (
	"context"
	"sync"
	"sync/atomic"
	"testing"
	"time"
)

func TestMemoryBus_FanOut(t *testing.T) {
	bus := NewMemoryBus()
	defer func() { _ = bus.Close() }()

	var got1, got2 atomic.Int64
	var wg sync.WaitGroup
	wg.Add(2)
	_, err := bus.Subscribe(Filter{}, func(_ context.Context, _ Event) error {
		got1.Add(1)
		wg.Done()
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	_, err = bus.Subscribe(Filter{Kinds: []Kind{KindObjectCreated}}, func(_ context.Context, _ Event) error {
		got2.Add(1)
		wg.Done()
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}

	if err := bus.Publish(context.Background(), Event{Kind: KindObjectCreated}); err != nil {
		t.Fatal(err)
	}
	wg.Wait()
	if got1.Load() != 1 || got2.Load() != 1 {
		t.Fatalf("expected fan-out 1/1, got %d/%d", got1.Load(), got2.Load())
	}
}

func TestMemoryBus_FilterByKind(t *testing.T) {
	bus := NewMemoryBus()
	defer func() { _ = bus.Close() }()

	var got atomic.Int64
	_, err := bus.Subscribe(Filter{Kinds: []Kind{KindObjectUpdated}}, func(_ context.Context, _ Event) error {
		got.Add(1)
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	for _, k := range []Kind{KindObjectCreated, KindObjectUpdated, KindObjectDeleted} {
		_ = bus.Publish(context.Background(), Event{Kind: k})
	}
	time.Sleep(50 * time.Millisecond)
	if got.Load() != 1 {
		t.Fatalf("expected 1 match, got %d", got.Load())
	}
}

func TestMemoryBus_Unsubscribe(t *testing.T) {
	bus := NewMemoryBus()
	defer func() { _ = bus.Close() }()

	var got atomic.Int64
	sub, err := bus.Subscribe(Filter{}, func(_ context.Context, _ Event) error {
		got.Add(1)
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}
	_ = bus.Publish(context.Background(), Event{Kind: KindAuditEmitted})
	time.Sleep(20 * time.Millisecond)
	_ = sub.Close()
	_ = bus.Publish(context.Background(), Event{Kind: KindAuditEmitted})
	time.Sleep(20 * time.Millisecond)
	if got.Load() != 1 {
		t.Fatalf("expected 1 (after unsubscribe), got %d", got.Load())
	}
}

func TestMemoryBus_BackpressureDropsNotBlock(t *testing.T) {
	bus := NewMemoryBus(WithBuffer(1))
	defer func() { _ = bus.Close() }()

	block := make(chan struct{})
	sub, err := bus.Subscribe(Filter{}, func(_ context.Context, _ Event) error {
		<-block
		return nil
	})
	if err != nil {
		t.Fatal(err)
	}

	done := make(chan struct{})
	go func() {
		for i := 0; i < 1000; i++ {
			_ = bus.Publish(context.Background(), Event{Kind: KindAuditEmitted})
		}
		close(done)
	}()
	select {
	case <-done:
	case <-time.After(2 * time.Second):
		t.Fatal("publish blocked under backpressure")
	}
	close(block)
	_ = sub.Close()
	if ms, ok := sub.(*memSub); ok && ms.Drops() == 0 {
		t.Fatalf("expected drops > 0 under buffer=1 with blocked handler")
	}
}

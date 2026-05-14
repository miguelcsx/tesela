// MemoryStore is an in-memory implementation suitable for tests and dev.
// Production code is expected to wire S3 or GCS backends.

package objectstore

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"sync"
	"time"
)

// MemoryStore is an in-memory Store. Safe for concurrent use.
type MemoryStore struct {
	mu      sync.RWMutex
	objects map[string]memoryObject
	baseURL string
	now     func() time.Time
}

type memoryObject struct {
	body []byte
	info ObjectInfo
}

// NewMemoryStore returns a fresh, empty MemoryStore. baseURL is used to
// construct fake signed URLs (the test harness can intercept these).
func NewMemoryStore(baseURL string) *MemoryStore {
	return &MemoryStore{
		objects: make(map[string]memoryObject),
		baseURL: baseURL,
		now:     time.Now,
	}
}

// Put implements Store.
func (s *MemoryStore) Put(_ context.Context, key string, body io.Reader, _ int64, opts PutOptions) (ObjectInfo, error) {
	buf, err := io.ReadAll(body)
	if err != nil {
		return ObjectInfo{}, err
	}
	info := ObjectInfo{
		Key: key, Size: int64(len(buf)), LastModified: s.now().UTC(),
		ContentType: opts.ContentType,
	}
	s.mu.Lock()
	s.objects[key] = memoryObject{body: buf, info: info}
	s.mu.Unlock()
	return info, nil
}

// Get implements Store.
func (s *MemoryStore) Get(_ context.Context, key string) (io.ReadCloser, ObjectInfo, error) {
	s.mu.RLock()
	obj, ok := s.objects[key]
	s.mu.RUnlock()
	if !ok {
		return nil, ObjectInfo{}, ErrNotFound
	}
	return io.NopCloser(bytes.NewReader(obj.body)), obj.info, nil
}

// Head implements Store.
func (s *MemoryStore) Head(_ context.Context, key string) (ObjectInfo, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	obj, ok := s.objects[key]
	if !ok {
		return ObjectInfo{}, ErrNotFound
	}
	return obj.info, nil
}

// Delete implements Store.
func (s *MemoryStore) Delete(_ context.Context, key string) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	if _, ok := s.objects[key]; !ok {
		return ErrNotFound
	}
	delete(s.objects, key)
	return nil
}

// SignedPutURL implements Store.
func (s *MemoryStore) SignedPutURL(_ context.Context, key string, _ SignedOptions) (string, error) {
	return fmt.Sprintf("%s/upload/%s", s.baseURL, key), nil
}

// SignedGetURL implements Store.
func (s *MemoryStore) SignedGetURL(_ context.Context, key string, _ SignedOptions) (string, error) {
	return fmt.Sprintf("%s/get/%s", s.baseURL, key), nil
}

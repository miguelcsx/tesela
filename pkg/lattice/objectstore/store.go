// Store is the unified object storage interface. Implementations live in
// per-backend sub-packages (s3, gcs, minio).

package objectstore

import (
	"context"
	"errors"
	"io"
	"time"
)

// ErrNotFound is returned when an object key does not exist.
var ErrNotFound = errors.New("objectstore: not found")

// PutOptions configures Put. ContentType is the most commonly used field;
// the rest are optional.
type PutOptions struct {
	ContentType string
	Metadata    map[string]string
}

// SignedOptions configures SignedPutURL / SignedGetURL.
type SignedOptions struct {
	Expires     time.Duration
	MaxBytes    int64
	ContentType string
}

// ObjectInfo is the metadata returned by Head/List.
type ObjectInfo struct {
	Key          string
	Size         int64
	LastModified time.Time
	ContentType  string
}

// Store is the contract every object storage backend implements.
type Store interface {
	// Put writes data at key. Returns the resulting ObjectInfo.
	Put(ctx context.Context, key string, body io.Reader, size int64, opts PutOptions) (ObjectInfo, error)
	// Get returns the body at key. Caller must Close.
	Get(ctx context.Context, key string) (io.ReadCloser, ObjectInfo, error)
	// Head returns metadata only.
	Head(ctx context.Context, key string) (ObjectInfo, error)
	// Delete removes the object at key.
	Delete(ctx context.Context, key string) error
	// SignedPutURL returns a presigned URL clients can PUT to directly.
	SignedPutURL(ctx context.Context, key string, opts SignedOptions) (string, error)
	// SignedGetURL returns a presigned URL clients can GET.
	SignedGetURL(ctx context.Context, key string, opts SignedOptions) (string, error)
}

// Package objectstore is the abstraction over cloud object storage (S3,
// GCS, MinIO). The upload pipeline talks to Store; concrete backends are
// composed at startup in cmd/*/main.go.
package objectstore

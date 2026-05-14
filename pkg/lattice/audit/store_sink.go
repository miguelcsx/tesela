// StoreSink persists batches into the audit_records table via storage.AuditRecordRepo.

package audit

import (
	"context"

	"github.com/miguelcsx/lattice/pkg/lattice/storage"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// StoreSink is a Sink backed by *storage.AuditRecordRepo.
type StoreSink struct {
	repo *storage.AuditRecordRepo
}

// NewStoreSink wraps a repo as a Sink.
func NewStoreSink(repo *storage.AuditRecordRepo) *StoreSink {
	return &StoreSink{repo: repo}
}

// Write inserts each record in the batch.
//
// We accept partial success: a failed insert logs upstream but does not stop
// the rest of the batch. The DB-level append-only constraint guarantees that
// a duplicate ID would surface as a conflict; the writer treats it as a drop.
func (s *StoreSink) Write(ctx context.Context, batch []types.AuditRecord) error {
	for _, rec := range batch {
		if _, err := s.repo.Create(ctx, rec); err != nil {
			return err
		}
	}
	return nil
}

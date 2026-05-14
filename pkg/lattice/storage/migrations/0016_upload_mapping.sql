-- +goose Up
-- +goose StatementBegin
ALTER TABLE uploads
    ADD COLUMN proposed_column_mapping JSONB,
    ADD COLUMN mapping_confidence FLOAT,
    ADD COLUMN mapping_proposed_at TIMESTAMPTZ,
    ADD COLUMN mapping_model_config JSONB;
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
ALTER TABLE uploads
    DROP COLUMN proposed_column_mapping,
    DROP COLUMN mapping_confidence,
    DROP COLUMN mapping_proposed_at,
    DROP COLUMN mapping_model_config;
-- +goose StatementEnd

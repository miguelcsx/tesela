-- +goose Up
-- +goose StatementBegin
ALTER TABLE assets
    ADD COLUMN metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN dependencies JSONB NOT NULL DEFAULT '[]'::jsonb;
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
ALTER TABLE assets
    DROP COLUMN dependencies,
    DROP COLUMN tags,
    DROP COLUMN metadata;
-- +goose StatementEnd

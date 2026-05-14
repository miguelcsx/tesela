-- +goose Up
-- +goose StatementBegin
ALTER TABLE uploads ADD COLUMN metadata JSONB;
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
ALTER TABLE uploads DROP COLUMN metadata;
-- +goose StatementEnd

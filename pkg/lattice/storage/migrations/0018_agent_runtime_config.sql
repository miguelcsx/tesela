-- +goose Up
-- +goose StatementBegin
ALTER TABLE agents
    ADD COLUMN context_sources JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN memory JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN planning JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN compaction JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN subagents JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN communication JSONB NOT NULL DEFAULT '{}'::jsonb;
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
ALTER TABLE agents
    DROP COLUMN communication,
    DROP COLUMN subagents,
    DROP COLUMN compaction,
    DROP COLUMN planning,
    DROP COLUMN memory,
    DROP COLUMN context_sources;
-- +goose StatementEnd

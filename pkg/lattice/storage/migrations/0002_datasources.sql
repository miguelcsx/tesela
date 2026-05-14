-- +goose Up
-- +goose StatementBegin
CREATE TABLE datasources (
    id                  UUID PRIMARY KEY,
    workspace_id        UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name            TEXT NOT NULL,
    display_name        TEXT NOT NULL,
    adapter_type        TEXT NOT NULL,
    config              JSONB NOT NULL DEFAULT '{}'::jsonb,
    sealed_credentials  BYTEA,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX datasources_workspace_idx ON datasources (workspace_id);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE datasources;
-- +goose StatementEnd

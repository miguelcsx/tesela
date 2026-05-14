-- +goose Up
-- +goose StatementBegin
CREATE TABLE object_types (
    id              UUID PRIMARY KEY,
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name        TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    primary_key     TEXT NOT NULL,
    source          JSONB NOT NULL,
    properties      JSONB NOT NULL DEFAULT '[]'::jsonb,
    environments    JSONB NOT NULL DEFAULT '[]'::jsonb,
    version         INTEGER NOT NULL DEFAULT 1,
    deprecated_at   TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX object_types_workspace_idx ON object_types (workspace_id);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE object_types;
-- +goose StatementEnd

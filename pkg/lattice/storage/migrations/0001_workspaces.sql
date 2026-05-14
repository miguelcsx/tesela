-- +goose Up
-- +goose StatementBegin
CREATE TABLE workspaces (
    id            UUID PRIMARY KEY,
    api_name      TEXT NOT NULL UNIQUE,
    display_name  TEXT NOT NULL,
    description   TEXT NOT NULL DEFAULT '',
    settings      JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX workspaces_api_name_idx ON workspaces (api_name);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE workspaces;
-- +goose StatementEnd

-- +goose Up
-- +goose StatementBegin
CREATE TABLE custom_tools (
    id              UUID PRIMARY KEY,
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name        TEXT NOT NULL,
    display_name    TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    kind            TEXT NOT NULL,
    input_schema    JSONB NOT NULL,
    output_schema   JSONB,
    sql_spec        JSONB,
    webhook         JSONB,
    composite       JSONB,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX custom_tools_workspace_idx ON custom_tools (workspace_id);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE custom_tools;
-- +goose StatementEnd

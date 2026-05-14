-- +goose Up
-- +goose StatementBegin
CREATE TABLE action_types (
    id                          UUID PRIMARY KEY,
    workspace_id                UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name                    TEXT NOT NULL,
    display_name                TEXT NOT NULL DEFAULT '',
    description                 TEXT NOT NULL DEFAULT '',
    subject                     TEXT NOT NULL DEFAULT '',
    input_schema                JSONB NOT NULL,
    output_schema               JSONB,
    permission_key              TEXT NOT NULL,
    idempotency_key_template    TEXT NOT NULL DEFAULT '',
    execution_mode              TEXT NOT NULL,
    handler                     JSONB NOT NULL,
    version                     INTEGER NOT NULL DEFAULT 1,
    deprecated_at               TIMESTAMPTZ,
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX action_types_workspace_idx ON action_types (workspace_id);
CREATE INDEX action_types_subject_idx ON action_types (workspace_id, subject);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE action_types;
-- +goose StatementEnd

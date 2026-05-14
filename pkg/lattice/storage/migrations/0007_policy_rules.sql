-- +goose Up
-- +goose StatementBegin
CREATE TABLE policy_rules (
    id              UUID PRIMARY KEY,
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name        TEXT NOT NULL,
    display_name    TEXT NOT NULL DEFAULT '',
    description     TEXT NOT NULL DEFAULT '',
    effect          TEXT NOT NULL CHECK (effect IN ('allow', 'deny')),
    roles           JSONB NOT NULL DEFAULT '[]'::jsonb,
    operations      JSONB NOT NULL DEFAULT '[]'::jsonb,
    object_type     TEXT NOT NULL DEFAULT '',
    action_type     TEXT NOT NULL DEFAULT '',
    row_filter      JSONB,
    conditions      JSONB NOT NULL DEFAULT '[]'::jsonb,
    redactions      JSONB NOT NULL DEFAULT '[]'::jsonb,
    priority        INTEGER NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX policy_rules_workspace_idx ON policy_rules (workspace_id);
CREATE INDEX policy_rules_object_type_idx ON policy_rules (workspace_id, object_type);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE policy_rules;
-- +goose StatementEnd

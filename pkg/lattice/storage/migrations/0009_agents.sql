-- +goose Up
-- +goose StatementBegin
CREATE TABLE agents (
    id                              UUID PRIMARY KEY,
    workspace_id                    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name                        TEXT NOT NULL,
    display_name                    TEXT NOT NULL DEFAULT '',
    description                     TEXT NOT NULL DEFAULT '',
    system_prompt                   TEXT NOT NULL DEFAULT '',
    model                           JSONB NOT NULL,
    from_object_types               JSONB NOT NULL DEFAULT '[]'::jsonb,
    from_link_types                 JSONB NOT NULL DEFAULT '[]'::jsonb,
    from_actions                    JSONB NOT NULL DEFAULT '[]'::jsonb,
    custom_tools                    JSONB NOT NULL DEFAULT '[]'::jsonb,
    allowed_roles                   JSONB NOT NULL DEFAULT '[]'::jsonb,
    limits                          JSONB NOT NULL DEFAULT '{}'::jsonb,
    require_approval_for_actions    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at                      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX agents_workspace_idx ON agents (workspace_id);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE agents;
-- +goose StatementEnd

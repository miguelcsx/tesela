-- +goose Up
-- +goose StatementBegin
CREATE TABLE link_types (
    id                  UUID PRIMARY KEY,
    workspace_id        UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name            TEXT NOT NULL,
    display_name        TEXT NOT NULL DEFAULT '',
    from_object_type    TEXT NOT NULL,
    to_object_type      TEXT NOT NULL,
    cardinality         TEXT NOT NULL,
    property_mappings   JSONB NOT NULL DEFAULT '[]'::jsonb,
    junction            JSONB,
    version             INTEGER NOT NULL DEFAULT 1,
    deprecated_at       TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX link_types_workspace_idx ON link_types (workspace_id);
CREATE INDEX link_types_from_idx ON link_types (workspace_id, from_object_type);
CREATE INDEX link_types_to_idx ON link_types (workspace_id, to_object_type);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE link_types;
-- +goose StatementEnd

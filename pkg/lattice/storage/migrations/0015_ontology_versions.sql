-- +goose Up
-- +goose StatementBegin
CREATE TABLE ontology_versions (
    id              UUID PRIMARY KEY,
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    snapshot        JSONB NOT NULL,
    created_by      TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    notes           TEXT NOT NULL DEFAULT '',
    UNIQUE (workspace_id, name)
);
CREATE INDEX ontology_versions_workspace_idx ON ontology_versions (workspace_id, created_at DESC);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE ontology_versions;
-- +goose StatementEnd

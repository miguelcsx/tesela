-- +goose Up
-- +goose StatementBegin
CREATE TABLE asset_versions (
    id              UUID PRIMARY KEY,
    workspace_id    UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    asset_id        UUID NOT NULL,
    upload_id       UUID NOT NULL REFERENCES uploads(id) ON DELETE CASCADE,
    row_count       BIGINT NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'staging',
    lineage         JSONB,
    metadata        JSONB,
    committed_at    TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX asset_versions_workspace_asset_idx ON asset_versions (workspace_id, asset_id);
CREATE INDEX asset_versions_upload_idx ON asset_versions (upload_id);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE asset_versions;
-- +goose StatementEnd

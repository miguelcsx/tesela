-- +goose Up
-- +goose StatementBegin
CREATE TABLE uploads (
    id                      UUID PRIMARY KEY,
    workspace_id            UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    asset                   TEXT NOT NULL,
    status                  TEXT NOT NULL,
    storage_url             TEXT NOT NULL DEFAULT '',
    signed_url              TEXT NOT NULL DEFAULT '',
    signed_url_expires      TIMESTAMPTZ,
    discovered_schema       JSONB,
    column_mapping          JSONB NOT NULL DEFAULT '[]'::jsonb,
    error_report_url        TEXT NOT NULL DEFAULT '',
    error_message           TEXT NOT NULL DEFAULT '',
    actor_user_id           TEXT NOT NULL,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX uploads_workspace_idx ON uploads (workspace_id);
CREATE INDEX uploads_asset_idx ON uploads (workspace_id, asset);
CREATE INDEX uploads_status_idx ON uploads (status) WHERE status NOT IN ('completed', 'failed');
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE uploads;
-- +goose StatementEnd

-- +goose Up
-- +goose StatementBegin
CREATE TABLE assets (
    id                          UUID PRIMARY KEY,
    workspace_id                UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    api_name                    TEXT NOT NULL,
    display_name                TEXT NOT NULL DEFAULT '',
    description                 TEXT NOT NULL DEFAULT '',
    properties                  JSONB NOT NULL DEFAULT '[]'::jsonb,
    quality_rules               JSONB NOT NULL DEFAULT '[]'::jsonb,
    sink                        JSONB NOT NULL,
    saved_column_mapping        JSONB NOT NULL DEFAULT '[]'::jsonb,
    unmapped_column_policy      TEXT NOT NULL DEFAULT 'warn',
    created_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at                  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, api_name)
);
CREATE INDEX assets_workspace_idx ON assets (workspace_id);

CREATE TABLE asset_versions (
    id          UUID PRIMARY KEY,
    asset_id    UUID NOT NULL REFERENCES assets(id) ON DELETE CASCADE,
    upload_id   UUID NOT NULL,
    row_count   BIGINT NOT NULL DEFAULT 0,
    committed   TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX asset_versions_asset_idx ON asset_versions (asset_id);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE asset_versions;
DROP TABLE assets;
-- +goose StatementEnd

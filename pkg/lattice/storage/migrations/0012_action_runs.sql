-- +goose Up
-- +goose StatementBegin
CREATE TABLE action_runs (
    id                  UUID PRIMARY KEY,
    workspace_id        UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    action_type         TEXT NOT NULL,
    idempotency_key     TEXT NOT NULL,
    subject             TEXT NOT NULL DEFAULT '',
    actor_user_id       TEXT NOT NULL,
    actor_roles         JSONB NOT NULL DEFAULT '[]'::jsonb,
    input               JSONB NOT NULL,
    output              JSONB,
    status              TEXT NOT NULL CHECK (status IN ('pending','running','done','failed','cancelled','awaiting_approval')),
    error_code          TEXT NOT NULL DEFAULT '',
    error_message       TEXT NOT NULL DEFAULT '',
    started_at          TIMESTAMPTZ,
    finished_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (workspace_id, idempotency_key)
);
CREATE INDEX action_runs_workspace_idx ON action_runs (workspace_id, created_at DESC);
CREATE INDEX action_runs_action_type_idx ON action_runs (workspace_id, action_type);
CREATE INDEX action_runs_status_idx ON action_runs (status) WHERE status IN ('pending', 'running');
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE action_runs;
-- +goose StatementEnd

-- +goose Up
-- +goose StatementBegin
CREATE TABLE audit_records (
    id                      UUID PRIMARY KEY,
    workspace_id            UUID NOT NULL,
    occurred_at             TIMESTAMPTZ NOT NULL DEFAULT now(),
    request_id              TEXT NOT NULL DEFAULT '',
    trace_id                TEXT NOT NULL DEFAULT '',
    actor_user_id           TEXT NOT NULL,
    actor_roles             JSONB NOT NULL DEFAULT '[]'::jsonb,
    operation               TEXT NOT NULL,
    resource_kind           TEXT NOT NULL,
    resource_api_name       TEXT NOT NULL DEFAULT '',
    subject_key             TEXT NOT NULL DEFAULT '',
    policy_decision         TEXT NOT NULL,
    matched_rules           JSONB NOT NULL DEFAULT '[]'::jsonb,
    redacted_properties     JSONB NOT NULL DEFAULT '[]'::jsonb,
    result_count            BIGINT NOT NULL DEFAULT 0,
    duration_ms             BIGINT NOT NULL DEFAULT 0,
    error_code              TEXT NOT NULL DEFAULT '',
    action_run_id           TEXT NOT NULL DEFAULT '',
    agent_run_id            TEXT NOT NULL DEFAULT '',
    metadata                JSONB NOT NULL DEFAULT '{}'::jsonb
);
CREATE INDEX audit_records_workspace_time_idx ON audit_records (workspace_id, occurred_at DESC);
CREATE INDEX audit_records_actor_idx ON audit_records (workspace_id, actor_user_id, occurred_at DESC);
CREATE INDEX audit_records_resource_idx ON audit_records (workspace_id, resource_kind, resource_api_name);
CREATE INDEX audit_records_request_idx ON audit_records (request_id) WHERE request_id <> '';
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE audit_records;
-- +goose StatementEnd

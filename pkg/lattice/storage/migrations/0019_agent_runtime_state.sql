-- +goose Up
-- +goose StatementBegin
ALTER TABLE agent_runs
    ADD COLUMN parent_run_id UUID,
    ADD COLUMN plan JSONB NOT NULL DEFAULT '{}'::jsonb,
    ADD COLUMN context_refs JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN memory_refs JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE TABLE agent_run_messages (
    id                  UUID PRIMARY KEY,
    agent_run_id        UUID NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    sequence            INTEGER NOT NULL,
    role                TEXT NOT NULL,
    kind                TEXT NOT NULL,
    content             TEXT NOT NULL DEFAULT '',
    name                TEXT NOT NULL DEFAULT '',
    tool_call_id        TEXT NOT NULL DEFAULT '',
    metadata            JSONB NOT NULL DEFAULT '{}'::jsonb,
    occurred_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agent_run_id, sequence)
);
CREATE INDEX agent_run_messages_run_idx ON agent_run_messages (agent_run_id, sequence);

CREATE TABLE agent_memory_records (
    id                  UUID PRIMARY KEY,
    workspace_id        UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    namespace           TEXT NOT NULL,
    scope               TEXT NOT NULL DEFAULT '',
    actor_user_id       TEXT NOT NULL DEFAULT '',
    agent               TEXT NOT NULL DEFAULT '',
    kind                TEXT NOT NULL,
    content             TEXT NOT NULL,
    summary             TEXT NOT NULL DEFAULT '',
    metadata            JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX agent_memory_records_lookup_idx ON agent_memory_records (workspace_id, namespace, created_at DESC);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE agent_memory_records;
DROP TABLE agent_run_messages;
ALTER TABLE agent_runs
    DROP COLUMN memory_refs,
    DROP COLUMN context_refs,
    DROP COLUMN plan,
    DROP COLUMN parent_run_id;
-- +goose StatementEnd

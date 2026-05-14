-- +goose Up
-- +goose StatementBegin
CREATE TABLE agent_runs (
    id                  UUID PRIMARY KEY,
    workspace_id        UUID NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    agent               TEXT NOT NULL,
    actor_user_id       TEXT NOT NULL,
    actor_roles         JSONB NOT NULL DEFAULT '[]'::jsonb,
    input               JSONB NOT NULL,
    final_response      TEXT NOT NULL DEFAULT '',
    status              TEXT NOT NULL,
    error_code          TEXT NOT NULL DEFAULT '',
    error_message       TEXT NOT NULL DEFAULT '',
    tokens_used         INTEGER NOT NULL DEFAULT 0,
    tool_call_count     INTEGER NOT NULL DEFAULT 0,
    cost_usd            DOUBLE PRECISION NOT NULL DEFAULT 0,
    started_at          TIMESTAMPTZ,
    finished_at         TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX agent_runs_workspace_idx ON agent_runs (workspace_id, created_at DESC);
CREATE INDEX agent_runs_agent_idx ON agent_runs (workspace_id, agent);

CREATE TABLE agent_run_tool_calls (
    id                  UUID PRIMARY KEY,
    agent_run_id        UUID NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    sequence            INTEGER NOT NULL,
    tool_name           TEXT NOT NULL,
    input               JSONB NOT NULL,
    output              JSONB,
    status              TEXT NOT NULL,
    policy_decision     TEXT NOT NULL,
    duration_ms         BIGINT NOT NULL DEFAULT 0,
    error_message       TEXT NOT NULL DEFAULT '',
    occurred_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (agent_run_id, sequence)
);
CREATE INDEX agent_run_tool_calls_run_idx ON agent_run_tool_calls (agent_run_id, sequence);
-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin
DROP TABLE agent_run_tool_calls;
DROP TABLE agent_runs;
-- +goose StatementEnd

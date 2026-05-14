// Package actions is the write-side runtime. It validates inputs against the
// action's JSON Schema, evaluates policy with the resolved subject, applies
// idempotency, persists the run record, dispatches the handler (CRUD,
// webhook, composite), records the result, and writes audit.
//
// Synchronous actions execute the handler inline; async actions enqueue a
// River job and return {run_id, status:pending}. The same Pipeline is used
// inside the worker so there is a single execution path for both modes.
package actions

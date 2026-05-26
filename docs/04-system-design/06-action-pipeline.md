# Action Pipeline

## Overview

The action pipeline manages the complete lifecycle of a mutation request: from input validation through policy check, idempotency deduplication, handler dispatch, result recording, and audit. Every action, regardless of handler type or execution mode, passes through this pipeline.

## Step 1: Actor Resolution

Identical to the query pipeline. The bearer token is validated and the actor is assembled from its claims.

## Step 2: Action Type Lookup

The action runtime loads the target action type from the ontology cache. If the action type is not found, the request fails immediately. The lookup retrieves the input schema, output schema, permission key, handler configuration, idempotency key template, and execution mode.

## Step 3: Input Validation

The action runtime validates the request body against the action type's declared JSON Schema. Validation checks required fields, field types, minimum and maximum constraints, enum membership, string patterns, and nested object structure. If validation fails, the request returns a structured validation error listing all failing constraints. No database access occurs.

## Step 4: Subject Resolution (if applicable)

If the action type has a subject object type, the runtime loads the subject object by primary key using the query pipeline (with full policy evaluation for the subject). If the subject is not found or the actor cannot read it, the request fails before any action check occurs.

## Step 5: Policy Evaluation

The policy engine checks whether the actor is permitted to execute the specific action type on the subject object type. The permission check uses the action type's permission key, not a generic write permission. Conditions on the policy rule (attribute matching, relationship checks, time windows) are evaluated here.

If the policy denies the action, the request fails with a forbidden error and an audit record is written. No further execution occurs.

## Step 6: Idempotency Check

The runtime evaluates the action type's idempotency key template against the current context (workspace identifier, subject identifier, actor identifier, and input fields). The resulting key is queried against the action runs table. If a run with the same key exists and has a terminal status (done or failed), the existing result is returned immediately without re-executing. If a run with the same key is in a non-terminal status (pending or running), the request waits or returns a conflict response.

## Step 7: Run Record Creation

A new action run record is created with a generated run identifier, the idempotency key, the actor, the subject identifier, the input (hashed for deduplication), and the initial status of pending. This record is written before any handler invocation.

## Step 8: Handler Dispatch

The run status is updated to running. Dispatch varies by handler type:

**crud_update, crud_create, crud_delete**: The runtime constructs a mutation and dispatches it to the adapter for the target object type. The mutation is executed within the adapter's transaction if supported.

**webhook**: The runtime makes an HTTP call to the configured URL, passing the action context (action type, subject, input, actor, run identifier) as the request body. The runtime respects the configured timeout and retries on configured status codes.

**composite**: The runtime executes each step in sequence. If a step fails and its on_failure policy is abort, the composite stops. If on_failure is skip, the next step executes. The outputs of earlier steps are available as inputs to later steps.

## Step 9: Result Recording

If the handler succeeds, the run status is set to done and the handler's output is recorded. If the handler fails, the run status is set to failed and the error is recorded. Either way, the run record is updated before the response is returned.

## Step 10: Audit and Response

An audit record is written with the run identifier, the actor, the action type, the subject, the policy decision, and the outcome. The response is returned to the client. For synchronous actions, this includes the handler output. For asynchronous actions, this includes the run identifier for subsequent status polling.

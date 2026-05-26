# Agent Pipeline

## Overview

The agent pipeline manages a complete agent run from initialization through tool execution to final response. The core principle is that the language model proposes actions; the runtime validates, authorizes, executes, and audits them. The model has no direct access to data or actions.

## Step 1: Run Initialization

The agent runtime loads the agent definition from the ontology cache. It creates a run record with a generated run identifier, the actor, the agent version, the model configuration, and the initial status of pending. The run record tracks all resource consumption (tool calls, tokens, monetary cost) and enforces the agent's declared limits.

## Step 2: Tool List Assembly

The runtime builds the tool list in three passes. First, it generates tools from the ontology for every object type (search and get tools), link type (traversal tools), and action type (execute tools) declared in the agent's from_object_types, from_link_types, and from_actions configuration. Second, it loads custom tool definitions for every tool listed in the agent's custom tools configuration. Third, it filters the combined list by the actor's policy — any tool that would result in a denied query or action for this actor is removed from the list. The resulting tool list contains only operations the actor is genuinely permitted to perform.

## Step 3: Context Assembly

The runtime builds the system prompt from two parts: the agent definition's declared system prompt, and an automatically generated ontology summary. The ontology summary describes the object types in the filtered tool list, their properties and their meanings, the link types available for traversal, and the actions available for execution. The actor's role and any access restrictions are noted so the model understands what it cannot do. This context is assembled from the live ontology and updates automatically when the ontology changes.

## Step 4: Model Call

The runtime sends the assembled context, the tool definitions in the model provider's required format, and the user's input to the language model. The model provider (Anthropic, OpenAI, Mistral, or Ollama) is determined by the agent definition's model configuration.

## Step 5: Tool Call Execution Loop

The model responds with either a final answer or one or more tool call proposals. For each proposed tool call, the runtime executes the following sequence:

**Schema validation**: The proposed tool input is validated against the tool's declared input schema. If validation fails, the error is returned to the model as a tool result so it can correct its approach.

**Policy check**: Even though the tool list was filtered at assembly time, the policy is checked again at execution time with the specific inputs. This catches cases where a filter condition depends on the specific object being accessed.

**Execution**: The tool is dispatched to the appropriate executor. Object query tools go through the query pipeline. Action tools go through the action pipeline. Custom SQL tools execute their query against the declared datasource. Webhook tools call the configured endpoint.

**Trace recording**: The tool call — its name, inputs, outputs, execution time, and policy decision — is recorded in the run trace.

**Limit check**: After each tool call, the runtime checks whether the agent's resource limits have been reached. If the maximum tool calls, maximum token budget, maximum cost, or timeout has been exceeded, the run is terminated and the model is not called again.

The loop continues until the model produces a final answer or a limit is reached.

## Step 6: Human Approval Gate (if configured)

If the agent definition declares require_approval_for_actions, any tool call that would execute an action pauses before dispatching. The runtime records the pending approval request. An external approval mechanism (a webhook, a notification to a review queue, or a manual confirmation flow) must authorize the execution before it proceeds. The run is in a waiting status until approval arrives or a timeout expires.

## Step 7: Run Completion

The run record is updated with the final status (completed or failed), the total token count, the total cost, the number of tool calls made, and the final response text. The full trace of tool calls is preserved for review and debugging.

## Step 8: Audit

An audit record is written for the entire agent run, referencing the run identifier. Individual tool calls are also audited as they execute — each query and action triggered by a tool call produces its own audit record, identical in structure to a directly-made API call.

// Tool list assembly from an ontology snapshot, filtered by policy.

package agents

import (
	"encoding/json"
	"fmt"

	"github.com/miguelcsx/lattice/pkg/lattice/modelproviders"
	"github.com/miguelcsx/lattice/pkg/lattice/policy"
	"github.com/miguelcsx/lattice/pkg/lattice/types"
)

// Tool is a single agent tool — the descriptor advertised to the model
// plus the executor used when the model issues a call.
type Tool struct {
	Descriptor modelproviders.Tool
	Kind       ToolKind
	ObjectType types.APIName
	LinkType   types.APIName
	ActionType types.APIName
	CustomTool types.APIName
	AgentRef   types.APIName
	Channel    string
}

// ToolKind discriminates how a tool is dispatched.
type ToolKind string

const (
	ToolKindSearch          ToolKind = "search"
	ToolKindGet             ToolKind = "get"
	ToolKindTraverse        ToolKind = "traverse"
	ToolKindExecute         ToolKind = "execute"
	ToolKindCustomSQL       ToolKind = "custom_sql"
	ToolKindCustomWebhook   ToolKind = "custom_webhook"
	ToolKindCustomCallback  ToolKind = "custom_callback"
	ToolKindCustomComposite ToolKind = "custom_composite"
	ToolKindMemoryRead      ToolKind = "memory_read"
	ToolKindMemoryWrite     ToolKind = "memory_write"
	ToolKindDelegateAgent   ToolKind = "delegate_agent"
	ToolKindCommunicate     ToolKind = "communicate"
)

// AssembleTools builds the tool list visible to the agent. Each ObjectType
// referenced in agent.FromObjectTypes contributes a search and get tool.
// Each LinkType contributes a traverse tool. Each ActionType contributes an
// execute tool. Each custom tool contributes one declarator. Tools the
// actor cannot use according to policy are filtered out.
func AssembleTools(snap *types.Ontology, agent types.Agent, actor types.Actor, eval *policy.Evaluator) ([]Tool, error) {
	out := make([]Tool, 0, 16)
	for _, otName := range agent.FromObjectTypes {
		ot, ok := snap.ObjectTypeByName(otName)
		if !ok {
			continue
		}
		if dec := eval.Evaluate(policy.Request{
			Actor: actor, Operation: types.OperationSearch,
			ResourceKind: types.KindObjectType, ResourceName: otName,
		}); dec.Allow {
			out = append(out, searchTool(ot))
		}
		if dec := eval.Evaluate(policy.Request{
			Actor: actor, Operation: types.OperationRead,
			ResourceKind: types.KindObjectType, ResourceName: otName,
		}); dec.Allow {
			out = append(out, getTool(ot))
		}
	}
	for _, ltName := range agent.FromLinkTypes {
		lt, ok := snap.LinkTypeByName(ltName)
		if !ok {
			continue
		}
		out = append(out, traverseTool(lt))
	}
	for _, atName := range agent.FromActions {
		at, ok := snap.ActionTypeByName(atName)
		if !ok {
			continue
		}
		if dec := eval.Evaluate(policy.Request{
			Actor: actor, Operation: types.OperationExecute,
			ResourceKind: types.KindActionType, ResourceName: atName,
		}); dec.Allow {
			out = append(out, executeTool(at))
		}
	}
	for _, ctName := range agent.CustomTools {
		ct, found := findCustomTool(snap, ctName)
		if !found {
			continue
		}
		out = append(out, customTool(ct))
	}
	if agent.Memory.Enabled {
		out = append(out, memoryReadTool(agent), memoryWriteTool(agent))
	}
	if agent.Subagents.Enabled {
		for _, ref := range agent.Subagents.AgentRefs {
			out = append(out, delegateTool(ref))
		}
	}
	for _, channel := range agent.Communication.Channels {
		out = append(out, communicateTool(channel))
	}
	return out, nil
}

func searchTool(ot types.ObjectType) Tool {
	schema := json.RawMessage(`{"type":"object","properties":{"filter":{"type":"object"},"limit":{"type":"integer","minimum":1,"maximum":500}}}`)
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        fmt.Sprintf("search_%s", ot.APIName),
			Description: fmt.Sprintf("Search %s objects by filter.", ot.APIName),
			Schema:      schema,
		},
		Kind: ToolKindSearch, ObjectType: ot.APIName,
	}
}

func getTool(ot types.ObjectType) Tool {
	schema := json.RawMessage(`{"type":"object","properties":{"primary_key":{"type":"string"}},"required":["primary_key"]}`)
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        fmt.Sprintf("get_%s", ot.APIName),
			Description: fmt.Sprintf("Fetch a single %s by primary key.", ot.APIName),
			Schema:      schema,
		},
		Kind: ToolKindGet, ObjectType: ot.APIName,
	}
}

func traverseTool(lt types.LinkType) Tool {
	schema := json.RawMessage(`{"type":"object","properties":{"source_key":{"type":"string"},"limit":{"type":"integer"}},"required":["source_key"]}`)
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        fmt.Sprintf("traverse_%s", lt.APIName),
			Description: fmt.Sprintf("Traverse the %s link from a source row.", lt.APIName),
			Schema:      schema,
		},
		Kind: ToolKindTraverse, LinkType: lt.APIName,
	}
}

func executeTool(at types.ActionType) Tool {
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        fmt.Sprintf("execute_%s", at.APIName),
			Description: at.Description,
			Schema:      at.InputSchema,
		},
		Kind: ToolKindExecute, ActionType: at.APIName,
	}
}

func customTool(ct types.CustomTool) Tool {
	kind := ToolKindCustomSQL
	if ct.Kind == types.CustomToolKindWebhook {
		kind = ToolKindCustomWebhook
	}
	if ct.Kind == types.CustomToolKindCallback {
		kind = ToolKindCustomCallback
	}
	if ct.Kind == types.CustomToolKindComposite {
		kind = ToolKindCustomComposite
	}
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        string(ct.APIName),
			Description: ct.Description,
			Schema:      ct.InputSchema,
		},
		Kind: kind, CustomTool: ct.APIName,
	}
}

func memoryReadTool(agent types.Agent) Tool {
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        "memory_search",
			Description: fmt.Sprintf("Read persistent memory for agent %s.", agent.APIName),
			Schema:      json.RawMessage(`{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","minimum":1,"maximum":100}}}`),
		},
		Kind: ToolKindMemoryRead,
	}
}

func memoryWriteTool(agent types.Agent) Tool {
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        "memory_write",
			Description: fmt.Sprintf("Persist a memory note for agent %s.", agent.APIName),
			Schema:      json.RawMessage(`{"type":"object","properties":{"content":{"type":"string"},"summary":{"type":"string"},"kind":{"type":"string"}},"required":["content"]}`),
		},
		Kind: ToolKindMemoryWrite,
	}
}

func delegateTool(agentRef types.APIName) Tool {
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        fmt.Sprintf("delegate_%s", agentRef),
			Description: fmt.Sprintf("Delegate a bounded task to subagent %s.", agentRef),
			Schema:      json.RawMessage(`{"type":"object","properties":{"input":{"type":"string"}},"required":["input"]}`),
		},
		Kind:     ToolKindDelegateAgent,
		AgentRef: agentRef,
	}
}

func communicateTool(channel types.AgentCommunicationChannel) Tool {
	return Tool{
		Descriptor: modelproviders.Tool{
			Name:        fmt.Sprintf("send_%s", channel.Name),
			Description: fmt.Sprintf("Send a message through communication channel %s.", channel.Name),
			Schema:      json.RawMessage(`{"type":"object","properties":{"message":{"type":"string"},"recipient":{"type":"string"}},"required":["message"]}`),
		},
		Kind:    ToolKindCommunicate,
		Channel: channel.Name,
	}
}

func findCustomTool(snap *types.Ontology, name types.APIName) (types.CustomTool, bool) {
	for _, ct := range snap.CustomTools {
		if ct.APIName == name {
			return ct, true
		}
	}
	return types.CustomTool{}, false
}

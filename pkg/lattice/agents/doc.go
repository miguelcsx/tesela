// Package agents is the AI agent runtime. Each Run starts from an actor +
// agent definition, assembles a tool list from the ontology (filtered by
// policy), and loops between the model provider and the tool executors
// until the model returns a final response or a limit is hit.
//
// Tools auto-derived from the ontology:
//
//	ObjectType.search   → calls query.Pipeline.Search
//	ObjectType.get      → calls query.Pipeline.Get
//	LinkType.traverse   → calls query.Pipeline.Traverse
//	ActionType.execute  → calls actions.Pipeline.Execute
//
// Custom tools (custom_tool.api_name) dispatch through the same Dispatcher
// the action runtime uses.
package agents

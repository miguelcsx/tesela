# Ontology Graph

## Two Levels of Graph

The ontology in Tesela forms a graph at two distinct levels: the schema graph and the instance graph. These are separate structures with different storage, different scale characteristics, and different use cases.

## The Schema Graph

The schema graph represents the ontology definition itself. Nodes are object types. Edges are link types, directed from the source object type to the target object type. This graph is small — it contains as many nodes as there are object types in the workspace, and as many edges as there are link types. It is stored in the metadata database and cached in memory as a directed multigraph structure.

The schema graph supports the following operations:

**Reachability analysis** determines which object types can be reached from a given starting object type within a given number of hops. This is used by the agent runtime to understand what traversal paths are available and by the UI to render navigation options.

**Impact analysis** identifies all entities that reference a given object type. When a team considers modifying or deprecating an object type, impact analysis shows which link types, action types, policy rules, and agent tool definitions would be affected.

**Lineage tracing** follows the chain from an object type back to the asset that backs it, and from the asset back to the raw datasource. This gives teams a complete picture of where an object's data originates.

**Shortest path** finds the minimum number of link traversals required to connect two object types. This is used for query planning and for helping users understand how to relate objects that appear unconnected.

**Topological sort** orders object types by their dependency relationships through link types. This is used for asset pipeline scheduling — assets that feed into other assets must be processed first.

## The Instance Graph

The instance graph represents the actual data: individual object instances and their specific link relationships. This graph is never stored by Tesela. It lives in the underlying data stores. When a link is traversed, Tesela generates a SQL join (or the equivalent for non-relational adapters) from the link type's property mappings and executes it against the relevant adapters.

The instance graph can be very large — millions or billions of nodes and edges — because it is distributed across the team's data stores. Traversal is bounded by the query limits declared in the workspace configuration.

## Multi-Hop Traversal

Tesela supports multi-hop traversal in two ways. The REST and GraphQL APIs expose single-hop traversal (follow one link type from one object). Multi-hop traversal is composed by clients making sequential requests, or by agents making sequential tool calls. For complex multi-hop patterns that need to execute in a single database round trip, teams define a custom SQL query tool that encodes the join chain explicitly.

## Graph-Derived Agent Tools

For every object type, the schema graph generates a set of agent tools: a search tool and a get tool. For every link type, the schema graph generates a traversal tool. The tool name and description are derived from the object type's API name and display name. This generation happens automatically when an agent run starts — no manual tool definition is needed for graph traversal.

## Optional Graph Database Adapter

For workspaces with highly connected domains where graph query patterns (finding all nodes within N hops, discovering connections between arbitrary nodes, cycle detection) are common, Tesela supports an optional graph database adapter. When configured, this adapter stores the instance graph in a native graph database and answers traversal queries using the graph database's query engine rather than generating SQL joins. The ontology schema graph remains in the metadata database regardless of this configuration.

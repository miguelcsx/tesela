//! Tool definitions for ontology-aware agents.

use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tesela_core::Error;

#[cfg(test)]
mod tests;

/// Approval policy advertised by a Tesela tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolApprovalPolicy {
    /// The tool can run without an explicit approval step.
    Never,
    /// The tool requires explicit approval with a user-visible reason.
    Required {
        /// Reason shown to the user.
        reason: &'static str,
    },
}

/// Side-effect classification for agent tool planning and policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolSideEffect {
    /// Reads existing state only.
    ReadOnly,
    /// Mutates user-visible UI state without changing persisted data.
    UserInterface,
    /// Mutates durable ontology/application data.
    DataMutation,
    /// Starts or controls external work.
    ExternalJob,
}

/// Empty tool input.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct EmptyArgs {}

/// Search records for one object type.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct SearchArgs {
    /// Object type API name.
    pub object_type: String,
    /// Optional page size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

/// Fetch one record by primary key.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct GetArgs {
    /// Object type API name.
    pub object_type: String,
    /// Primary key.
    pub id: String,
}

/// Aggregate records for one object type.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct AggregateArgs {
    /// Object type API name.
    pub object_type: String,
    /// Group-by property names.
    #[serde(default)]
    pub group_by: Vec<String>,
    /// Aggregate expressions.
    #[serde(default)]
    pub aggregations: Vec<AggregateInput>,
}

/// Aggregate expression accepted by the ontology aggregate tool.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct AggregateInput {
    /// Aggregate function.
    pub function: AggregateFunctionInput,
    /// Property to aggregate for non-count functions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    /// Output alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
}

/// Aggregate function accepted by tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AggregateFunctionInput {
    /// Count records.
    Count,
    /// Sum numeric values.
    Sum,
    /// Average numeric values.
    Avg,
    /// Minimum value.
    Min,
    /// Maximum value.
    Max,
}

/// Resolve a saved object set.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ObjectSetResolveArgs {
    /// Object set API name.
    pub name: String,
}

/// Compose saved object sets.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ObjectSetComposeArgs {
    /// Object set API names.
    pub names: Vec<String>,
    /// Composition operation.
    pub op: ObjectSetComposeOp,
}

/// Object set composition operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObjectSetComposeOp {
    /// Union all object sets.
    Union,
    /// Intersect all object sets.
    Intersect,
    /// Subtract later sets from the first set.
    Subtract,
}

/// Traverse a link from a source record.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct TraverseArgs {
    /// Link type API name.
    pub link: String,
    /// Source record primary key.
    pub source_id: String,
    /// Optional page size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
}

/// Describe one action.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct ActionDescribeArgs {
    /// Action API name.
    pub action: String,
}

/// Ontology tool exposed by Tesela runtimes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OntologyTool {
    /// Return the active ontology spec.
    Spec,
    /// Search records for any object type.
    Search,
    /// Fetch one record by object type and primary key.
    Get,
    /// Aggregate records.
    Aggregate,
    /// Resolve a named object set.
    ObjectSetResolve,
    /// Compose named object sets.
    ObjectSetCompose,
    /// List ontology link types.
    LinksList,
    /// Follow a named ontology link.
    Traverse,
    /// List ontology actions.
    ActionsList,
    /// Describe one ontology action.
    ActionDescribe,
}

const SPEC_TOOL_NAME: &str = "tesela.spec";
const SEARCH_TOOL_NAME: &str = "tesela.search";
const GET_TOOL_NAME: &str = "tesela.get";
const AGGREGATE_TOOL_NAME: &str = "tesela.aggregate";
const OBJECT_SET_RESOLVE_TOOL_NAME: &str = "tesela.object_set.resolve";
const OBJECT_SET_COMPOSE_TOOL_NAME: &str = "tesela.object_set.compose";
const LINKS_LIST_TOOL_NAME: &str = "tesela.links.list";
const TRAVERSE_TOOL_NAME: &str = "tesela.traverse";
const ACTIONS_LIST_TOOL_NAME: &str = "tesela.actions.list";
const ACTION_DESCRIBE_TOOL_NAME: &str = "tesela.action.describe";

impl OntologyTool {
    /// All built-in ontology tools in stable registration order.
    pub const ALL: &'static [Self] = &[
        Self::Spec,
        Self::Search,
        Self::Get,
        Self::Aggregate,
        Self::ObjectSetResolve,
        Self::ObjectSetCompose,
        Self::LinksList,
        Self::Traverse,
        Self::ActionsList,
        Self::ActionDescribe,
    ];

    /// Parse a provider-visible tool name.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            SPEC_TOOL_NAME => Some(Self::Spec),
            SEARCH_TOOL_NAME => Some(Self::Search),
            GET_TOOL_NAME => Some(Self::Get),
            AGGREGATE_TOOL_NAME => Some(Self::Aggregate),
            OBJECT_SET_RESOLVE_TOOL_NAME => Some(Self::ObjectSetResolve),
            OBJECT_SET_COMPOSE_TOOL_NAME => Some(Self::ObjectSetCompose),
            LINKS_LIST_TOOL_NAME => Some(Self::LinksList),
            TRAVERSE_TOOL_NAME => Some(Self::Traverse),
            ACTIONS_LIST_TOOL_NAME => Some(Self::ActionsList),
            ACTION_DESCRIBE_TOOL_NAME => Some(Self::ActionDescribe),
            _ => None,
        }
    }

    /// Provider-visible function name.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Spec => SPEC_TOOL_NAME,
            Self::Search => SEARCH_TOOL_NAME,
            Self::Get => GET_TOOL_NAME,
            Self::Aggregate => AGGREGATE_TOOL_NAME,
            Self::ObjectSetResolve => OBJECT_SET_RESOLVE_TOOL_NAME,
            Self::ObjectSetCompose => OBJECT_SET_COMPOSE_TOOL_NAME,
            Self::LinksList => LINKS_LIST_TOOL_NAME,
            Self::Traverse => TRAVERSE_TOOL_NAME,
            Self::ActionsList => ACTIONS_LIST_TOOL_NAME,
            Self::ActionDescribe => ACTION_DESCRIBE_TOOL_NAME,
        }
    }

    /// Human-readable tool description.
    #[must_use]
    pub fn description(self) -> &'static str {
        match self {
            Self::Spec => "Return the active ontology spec.",
            Self::Search => "Search records for any object type.",
            Self::Get => "Fetch one record by object type and primary key.",
            Self::Aggregate => "Aggregate records by count, sum, avg, min, or max.",
            Self::ObjectSetResolve => "Resolve a named object set.",
            Self::ObjectSetCompose => {
                "Compose named object sets with union, intersect, or subtract."
            }
            Self::LinksList => "List ontology link types.",
            Self::Traverse => "Follow a named ontology link from a source record.",
            Self::ActionsList => "List available ontology actions.",
            Self::ActionDescribe => "Describe one ontology action.",
        }
    }

    /// Short display title.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Self::Spec => "Read Ontology Spec",
            Self::Search => "Search Objects",
            Self::Get => "Get Object",
            Self::Aggregate => "Aggregate Objects",
            Self::ObjectSetResolve => "Resolve Object Set",
            Self::ObjectSetCompose => "Compose Object Sets",
            Self::LinksList => "List Links",
            Self::Traverse => "Traverse Link",
            Self::ActionsList => "List Actions",
            Self::ActionDescribe => "Describe Action",
        }
    }

    /// Prompt guidance for agents.
    #[must_use]
    pub fn prompt_hint(self) -> &'static str {
        match self {
            Self::Spec => {
                "Use when you need the ontology contract before choosing object types or links."
            }
            Self::Search => {
                "Use scoped filters whenever possible; prefer get or traverse when you already have IDs."
            }
            Self::Get => "Use when the user or frontend context provides an object ID.",
            Self::Aggregate => "Use for numeric summaries over one object type.",
            Self::ObjectSetResolve => "Use saved object sets instead of recreating common filters.",
            Self::ObjectSetCompose => {
                "Use to combine saved object sets without loading unrelated records."
            }
            Self::LinksList => "Use before traverse when you need to discover valid graph edges.",
            Self::Traverse => {
                "Use to follow declared ontology links from a verified source record."
            }
            Self::ActionsList => "Use before proposing ontology actions.",
            Self::ActionDescribe => {
                "Use before executing or explaining a specific action contract."
            }
        }
    }

    /// Approval policy for this tool.
    #[must_use]
    pub fn approval(self) -> ToolApprovalPolicy {
        ToolApprovalPolicy::Never
    }

    /// Side-effect classification.
    #[must_use]
    pub fn side_effect(self) -> ToolSideEffect {
        ToolSideEffect::ReadOnly
    }

    /// JSON schema for the typed tool input.
    pub fn input_schema(self) -> Result<Value, Error> {
        match self {
            Self::Spec | Self::LinksList | Self::ActionsList => schema::<EmptyArgs>(),
            Self::Search => schema::<SearchArgs>(),
            Self::Get => schema::<GetArgs>(),
            Self::Aggregate => schema::<AggregateArgs>(),
            Self::ObjectSetResolve => schema::<ObjectSetResolveArgs>(),
            Self::ObjectSetCompose => schema::<ObjectSetComposeArgs>(),
            Self::Traverse => schema::<TraverseArgs>(),
            Self::ActionDescribe => schema::<ActionDescribeArgs>(),
        }
    }

    /// Build a self-contained tool definition.
    pub fn definition(self) -> Result<OntologyToolDefinition, Error> {
        Ok(OntologyToolDefinition {
            tool: self,
            name: self.name(),
            title: self.title(),
            description: self.description(),
            prompt_hint: self.prompt_hint(),
            approval: self.approval(),
            side_effect: self.side_effect(),
            input_schema: self.input_schema()?,
        })
    }
}

/// Self-contained ontology tool definition.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OntologyToolDefinition {
    /// Typed tool identifier.
    pub tool: OntologyTool,
    /// Provider-visible name.
    pub name: &'static str,
    /// Short display title.
    pub title: &'static str,
    /// Provider-visible description.
    pub description: &'static str,
    /// Prompt guidance for agents.
    pub prompt_hint: &'static str,
    /// Approval policy.
    pub approval: ToolApprovalPolicy,
    /// Side-effect classification.
    pub side_effect: ToolSideEffect,
    /// JSON schema for arguments.
    pub input_schema: Value,
}

/// Built-in ontology tool definitions.
pub fn ontology_tool_definitions() -> Result<Vec<OntologyToolDefinition>, Error> {
    OntologyTool::ALL
        .iter()
        .copied()
        .map(OntologyTool::definition)
        .collect()
}

fn schema<T>() -> Result<Value, Error>
where
    T: JsonSchema,
{
    serde_json::to_value(schema_for!(T))
        .map_err(|error| Error::validation(format!("tool schema serialization failed: {error}")))
}

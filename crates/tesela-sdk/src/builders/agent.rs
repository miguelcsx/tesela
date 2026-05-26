use super::prelude::*;

/// Builder for [`Agent`].
pub struct AgentBuilder {
    api_name: ApiName,
    display: Option<String>,
    description: Option<String>,
    model: Option<String>,
    model_provider: Option<String>,
    instructions: Option<String>,
    allowed_tools: Vec<ApiName>,
    custom_tools: Vec<ApiName>,
    context_sources: Vec<ContextSource>,
    memory: Option<AgentMemory>,
    limits: Option<AgentLimits>,
    requires_approval: Option<bool>,
    capabilities: Vec<String>,
    output_schema: Option<Value>,
    output_object_type: Option<ApiName>,
    deprecated_at: Option<String>,
    metadata: Option<std::collections::BTreeMap<String, Value>>,
}

impl AgentBuilder {
    /// Create a new agent builder.
    pub fn new(api_name: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            display: None,
            description: None,
            model: None,
            model_provider: None,
            instructions: None,
            allowed_tools: Vec::new(),
            custom_tools: Vec::new(),
            context_sources: Vec::new(),
            memory: None,
            limits: None,
            requires_approval: None,
            capabilities: Vec::new(),
            output_schema: None,
            output_object_type: None,
            deprecated_at: None,
            metadata: None,
        }
    }

    /// Set display name.
    pub fn display(mut self, name: impl Into<String>) -> Self {
        self.display = Some(name.into());
        self
    }

    /// Set description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the model identifier.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Set the model provider.
    pub fn model_provider(mut self, provider: impl Into<String>) -> Self {
        self.model_provider = Some(provider.into());
        self
    }

    /// Set the system instructions.
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    /// Allow a named tool.
    pub fn allow_tool(mut self, tool: impl AsRef<str>) -> Self {
        self.allowed_tools
            .push(ApiName::new_unchecked(tool.as_ref()));
        self
    }

    /// Allow a custom tool.
    pub fn custom_tool(mut self, tool: impl AsRef<str>) -> Self {
        self.custom_tools
            .push(ApiName::new_unchecked(tool.as_ref()));
        self
    }

    /// Add a context source.
    pub fn context_source(
        mut self,
        name: impl Into<String>,
        kind: impl Into<String>,
        r#ref: Option<impl Into<String>>,
    ) -> Self {
        self.context_sources.push(ContextSource {
            name: name.into(),
            kind: kind.into(),
            r#ref: r#ref.map(Into::into),
            description: None,
            max_items: None,
            metadata: None,
        });
        self
    }

    /// Set memory configuration.
    pub fn memory(mut self, enabled: bool) -> Self {
        self.memory = Some(AgentMemory {
            enabled: Some(enabled),
            namespace: None,
            scope: None,
        });
        self
    }

    /// Set memory with namespace and scope.
    pub fn memory_with(
        mut self,
        enabled: bool,
        namespace: impl Into<String>,
        scope: impl Into<String>,
    ) -> Self {
        self.memory = Some(AgentMemory {
            enabled: Some(enabled),
            namespace: Some(namespace.into()),
            scope: Some(scope.into()),
        });
        self
    }

    /// Set execution limits.
    pub fn limits(mut self, max_tool_calls: i32, max_tokens: i32, timeout_seconds: i32) -> Self {
        self.limits = Some(AgentLimits {
            max_tool_calls: Some(max_tool_calls),
            max_tokens: Some(max_tokens),
            max_cost_usd: None,
            timeout_seconds: Some(timeout_seconds),
            temperature: None,
            token_budget: None,
        });
        self
    }

    /// Set token budget.
    pub fn token_budget(mut self, budget: u32) -> Self {
        let mut limits = self.limits.unwrap_or(AgentLimits {
            max_tool_calls: None,
            max_tokens: None,
            max_cost_usd: None,
            timeout_seconds: None,
            temperature: None,
            token_budget: None,
        });
        limits.token_budget = Some(budget);
        self.limits = Some(limits);
        self
    }

    /// Require human approval.
    pub fn requires_approval(mut self) -> Self {
        self.requires_approval = Some(true);
        self
    }

    /// Add a capability tag.
    pub fn capability(mut self, c: impl Into<String>) -> Self {
        self.capabilities.push(c.into());
        self
    }

    /// Set output JSON schema.
    pub fn output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Set output object type.
    pub fn output_object_type(mut self, ot: impl AsRef<str>) -> Self {
        self.output_object_type = Some(ApiName::new_unchecked(ot.as_ref()));
        self
    }

    /// Set deprecation timestamp.
    pub fn deprecated_at(mut self, v: impl Into<String>) -> Self {
        self.deprecated_at = Some(v.into());
        self
    }

    /// Set metadata key/value.
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Build the [`Agent`].
    pub fn build(self) -> Agent {
        Agent {
            api_name: self.api_name,
            display: self.display,
            description: self.description,
            model: self.model,
            model_provider: self.model_provider,
            instructions: self.instructions,
            allowed_tools: self.allowed_tools,
            custom_tools: self.custom_tools,
            context_sources: self.context_sources,
            memory: self.memory,
            limits: self.limits,
            requires_approval: self.requires_approval,
            deprecated_at: self.deprecated_at,
            metadata: self.metadata,
            capabilities: self.capabilities,
            output_schema: self.output_schema,
            output_object_type: self.output_object_type,
        }
    }
}

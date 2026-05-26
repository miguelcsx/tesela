//! Fluent builder types for ontology elements.

use tesela_core::{ApiName, DataType, LinkCardinality, Operation, PolicyEffect, Value};
use tesela_ir::{
    ActionHandler, ActionType, Agent, AgentLimits, AgentMemory, AggregateFunction,
    AggregateMeasure, AggregateView, ArtifactType, CapabilityGrant, ClassificationConfig, Computed,
    ContextSource, Datasource, EventType, Index, JobType, JunctionConfig, LifecycleConfig,
    LinkMapping, LinkSource, LinkType, ObjectSource, ObjectType, PolicyRule, Property, QualityRule,
    QualityRuleRef, ScoringConfig, SpatialExtent, TemporalConfig, TimeBucket, UploadFlow,
};

// ---------------------------------------------------------------------------
// ObjectTypeBuilder
// ---------------------------------------------------------------------------

/// Builder for [`ObjectType`].
pub struct ObjectTypeBuilder {
    api_name: ApiName,
    display: Option<String>,
    description: Option<String>,
    datasource: ApiName,
    resource: String,
    primary_key: ApiName,
    properties: Vec<Property>,
    traits: Vec<ApiName>,
    tags: Vec<String>,
    metadata: Option<std::collections::BTreeMap<String, Value>>,
    indexes: Vec<Index>,
    temporal: Option<TemporalConfig>,
    lifecycle: Option<LifecycleConfig>,
    scoring: Option<ScoringConfig>,
    classification: Option<ClassificationConfig>,
    quality_rules: Vec<QualityRule>,
    lineage: Vec<tesela_ir::LineageEdge>,
    deprecated_at: Option<String>,
}

impl ObjectTypeBuilder {
    /// Create a new builder.
    ///
    /// The default datasource is `"memory"` (in-process, non-persistent).
    /// Call [`.datasource()`](Self::datasource) to set a production backend
    /// before deploying.
    pub fn new(api_name: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            display: None,
            description: None,
            datasource: ApiName::new_unchecked("memory"),
            resource: String::new(),
            primary_key: ApiName::new_unchecked("id"),
            properties: Vec::new(),
            traits: Vec::new(),
            tags: Vec::new(),
            metadata: None,
            indexes: Vec::new(),
            temporal: None,
            lifecycle: None,
            scoring: None,
            classification: None,
            quality_rules: Vec::new(),
            lineage: Vec::new(),
            deprecated_at: None,
        }
    }

    /// Set the display name.
    pub fn display(mut self, name: impl Into<String>) -> Self {
        self.display = Some(name.into());
        self
    }

    /// Set the description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the datasource API name.
    pub fn datasource(mut self, ds: impl AsRef<str>) -> Self {
        self.datasource = ApiName::new_unchecked(ds.as_ref());
        self
    }

    /// Set the physical resource (table/collection name).
    pub fn resource(mut self, r: impl Into<String>) -> Self {
        self.resource = r.into();
        self
    }

    /// Set the primary key property name.
    pub fn primary_key(mut self, pk: impl AsRef<str>) -> Self {
        self.primary_key = ApiName::new_unchecked(pk.as_ref());
        self
    }

    /// Add a property.
    pub fn property(mut self, p: Property) -> Self {
        self.properties.push(p);
        self
    }

    /// Add a trait reference.
    pub fn trait_def(mut self, name: impl AsRef<str>) -> Self {
        self.traits.push(ApiName::new_unchecked(name.as_ref()));
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set metadata key/value.
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Add an index.
    pub fn index(
        mut self,
        api_name: impl AsRef<str>,
        properties: Vec<impl AsRef<str>>,
        unique: bool,
    ) -> Self {
        self.indexes.push(Index {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            properties: properties
                .into_iter()
                .map(|p| ApiName::new_unchecked(p.as_ref()))
                .collect(),
            unique: if unique { Some(true) } else { None },
        });
        self
    }

    /// Set temporal configuration.
    pub fn temporal(mut self, cfg: TemporalConfig) -> Self {
        self.temporal = Some(cfg);
        self
    }

    /// Set lifecycle configuration.
    pub fn lifecycle(mut self, cfg: LifecycleConfig) -> Self {
        self.lifecycle = Some(cfg);
        self
    }

    /// Set scoring configuration.
    pub fn scoring(mut self, cfg: ScoringConfig) -> Self {
        self.scoring = Some(cfg);
        self
    }

    /// Set classification configuration.
    pub fn classification(mut self, cfg: ClassificationConfig) -> Self {
        self.classification = Some(cfg);
        self
    }

    /// Add a quality rule.
    pub fn quality_rule(mut self, rule: QualityRule) -> Self {
        self.quality_rules.push(rule);
        self
    }

    /// Add a lineage edge.
    pub fn lineage(mut self, edge: tesela_ir::LineageEdge) -> Self {
        self.lineage.push(edge);
        self
    }

    /// Set deprecation timestamp.
    pub fn deprecated_at(mut self, v: impl Into<String>) -> Self {
        self.deprecated_at = Some(v.into());
        self
    }

    /// Build the [`ObjectType`].
    pub fn build(self) -> ObjectType {
        let api_name = self.api_name.to_string();
        ObjectType {
            api_name: self.api_name,
            display: self.display,
            description: self.description,
            source: ObjectSource {
                datasource: self.datasource,
                resource: Some(if self.resource.is_empty() {
                    api_name
                } else {
                    self.resource
                }),
            },
            primary_key: self.primary_key,
            properties: self.properties,
            traits: self.traits,
            tags: self.tags,
            metadata: self.metadata,
            indexes: self.indexes,
            temporal: self.temporal,
            lifecycle: self.lifecycle,
            scoring: self.scoring,
            classification: self.classification,
            quality_rules: self.quality_rules,
            lineage: self.lineage,
            deprecated_at: self.deprecated_at,
        }
    }
}

// ---------------------------------------------------------------------------
// PropertyBuilder
// ---------------------------------------------------------------------------

/// Builder for [`Property`].
pub struct PropertyBuilder {
    api_name: ApiName,
    data_type: DataType,
    display: Option<String>,
    description: Option<String>,
    required: bool,
    unique: bool,
    indexed: bool,
    default: Option<Value>,
    computed: Option<Computed>,
    source_column: Option<String>,
    allowed_values: Option<Vec<Value>>,
    sort_order: Option<String>,
    tags: Vec<String>,
    markings: Vec<String>,
    encrypted: Option<bool>,
    quality: Vec<QualityRuleRef>,
    metadata: Option<std::collections::BTreeMap<String, Value>>,
}

impl PropertyBuilder {
    /// Create a new property builder.
    pub fn new(api_name: impl AsRef<str>, data_type: DataType) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            data_type,
            display: None,
            description: None,
            required: false,
            unique: false,
            indexed: false,
            default: None,
            computed: None,
            source_column: None,
            allowed_values: None,
            sort_order: None,
            tags: Vec::new(),
            markings: Vec::new(),
            encrypted: None,
            quality: Vec::new(),
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

    /// Mark as required.
    pub fn required(mut self, r: bool) -> Self {
        self.required = r;
        self
    }

    /// Mark as unique.
    pub fn unique(mut self, u: bool) -> Self {
        self.unique = u;
        self
    }

    /// Mark as indexed.
    pub fn indexed(mut self, i: bool) -> Self {
        self.indexed = i;
        self
    }

    /// Set default value.
    pub fn default(mut self, v: Value) -> Self {
        self.default = Some(v);
        self
    }

    /// Set computed expression.
    pub fn computed(mut self, language: impl Into<String>, expression: impl Into<String>) -> Self {
        self.computed = Some(Computed {
            language: language.into(),
            expression: expression.into(),
        });
        self
    }

    /// Set source column mapping.
    pub fn source_column(mut self, col: impl Into<String>) -> Self {
        self.source_column = Some(col.into());
        self
    }

    /// Set allowed enum values.
    pub fn allowed_values(mut self, vals: Vec<Value>) -> Self {
        self.allowed_values = Some(vals);
        self
    }

    /// Set sort order hint.
    pub fn sort_order(mut self, order: impl Into<String>) -> Self {
        self.sort_order = Some(order.into());
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add a marking.
    pub fn marking(mut self, marking: impl Into<String>) -> Self {
        self.markings.push(marking.into());
        self
    }

    /// Mark as encrypted at rest.
    pub fn encrypted(mut self, v: bool) -> Self {
        self.encrypted = Some(v);
        self
    }

    /// Add a quality rule reference.
    pub fn quality(mut self, api_name: impl AsRef<str>, kind: impl Into<String>) -> Self {
        self.quality.push(QualityRuleRef {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            kind: kind.into(),
            args: None,
        });
        self
    }

    /// Set metadata key/value.
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Build the [`Property`].
    pub fn build(self) -> Property {
        Property {
            api_name: self.api_name,
            display: self.display,
            description: self.description,
            data_type: self.data_type,
            nullable: if self.required { Some(false) } else { None },
            unique: if self.unique { Some(true) } else { None },
            indexed: if self.indexed { Some(true) } else { None },
            default: self.default,
            computed: self.computed,
            source_column: self.source_column,
            allowed_values: self.allowed_values,
            sort_order: self.sort_order,
            tags: self.tags,
            markings: self.markings,
            encrypted: self.encrypted,
            quality: self.quality,
            metadata: self.metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// LinkBuilder
// ---------------------------------------------------------------------------

/// Builder for [`LinkType`].
pub struct LinkBuilder {
    api_name: ApiName,
    display: Option<String>,
    from: ApiName,
    to: ApiName,
    cardinality: LinkCardinality,
    source: Option<LinkSource>,
    mappings: Vec<LinkMapping>,
    junction: Option<JunctionConfig>,
    deprecated_at: Option<String>,
    metadata: Option<std::collections::BTreeMap<String, Value>>,
}

impl LinkBuilder {
    /// Create a new link builder.
    pub fn new(api_name: impl AsRef<str>, from: impl AsRef<str>, to: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            display: None,
            from: ApiName::new_unchecked(from.as_ref()),
            to: ApiName::new_unchecked(to.as_ref()),
            cardinality: LinkCardinality::ManyToMany,
            source: None,
            mappings: Vec::new(),
            junction: None,
            deprecated_at: None,
            metadata: None,
        }
    }

    /// Set display name.
    pub fn display(mut self, name: impl Into<String>) -> Self {
        self.display = Some(name.into());
        self
    }

    /// Set cardinality.
    pub fn cardinality(mut self, c: LinkCardinality) -> Self {
        self.cardinality = c;
        self
    }

    /// Add a property join mapping.
    pub fn mapping(mut self, from_property: impl AsRef<str>, to_property: impl AsRef<str>) -> Self {
        self.mappings.push(LinkMapping {
            from_property: ApiName::new_unchecked(from_property.as_ref()),
            to_property: ApiName::new_unchecked(to_property.as_ref()),
        });
        self
    }

    /// Set physical source mapping.
    pub fn source(mut self, datasource: impl AsRef<str>, resource: impl Into<String>) -> Self {
        self.source = Some(LinkSource {
            datasource: Some(ApiName::new_unchecked(datasource.as_ref())),
            resource: Some(resource.into()),
        });
        self
    }

    /// Set junction table configuration.
    pub fn junction(
        mut self,
        datasource: impl AsRef<str>,
        resource: impl Into<String>,
        from_column: impl Into<String>,
        to_column: impl Into<String>,
    ) -> Self {
        self.junction = Some(JunctionConfig {
            datasource: ApiName::new_unchecked(datasource.as_ref()),
            resource: resource.into(),
            from_column: from_column.into(),
            to_column: to_column.into(),
            properties: Vec::new(),
        });
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

    /// Build the [`LinkType`].
    pub fn build(self) -> LinkType {
        LinkType {
            api_name: self.api_name,
            display: self.display,
            from: self.from,
            to: self.to,
            cardinality: self.cardinality,
            source: self.source,
            mappings: self.mappings,
            junction: self.junction,
            deprecated_at: self.deprecated_at,
            metadata: self.metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// ActionBuilder
// ---------------------------------------------------------------------------

/// Builder for [`ActionType`].
pub struct ActionBuilder {
    api_name: ApiName,
    display: Option<String>,
    description: Option<String>,
    subject: Option<ApiName>,
    handler_kind: String,
    handler_target: Option<String>,
    handler_config: Option<std::collections::BTreeMap<String, Value>>,
    input_schema: Option<Value>,
    output_schema: Option<Value>,
    mode: Option<String>,
    risk_level: Option<String>,
    idempotency_key: Option<String>,
    deprecated_at: Option<String>,
    metadata: Option<std::collections::BTreeMap<String, Value>>,
}

impl ActionBuilder {
    /// Create a new action builder.
    pub fn new(api_name: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            display: None,
            description: None,
            subject: None,
            handler_kind: "callback".to_string(),
            handler_target: None,
            handler_config: None,
            input_schema: None,
            output_schema: None,
            mode: None,
            risk_level: None,
            idempotency_key: None,
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

    /// Set subject object type.
    pub fn subject(mut self, ot: impl AsRef<str>) -> Self {
        self.subject = Some(ApiName::new_unchecked(ot.as_ref()));
        self
    }

    /// Set handler kind and optional target.
    pub fn handler(mut self, kind: impl Into<String>, target: Option<String>) -> Self {
        self.handler_kind = kind.into();
        self.handler_target = target;
        self
    }

    /// Set handler config key/value.
    pub fn handler_config(mut self, key: impl Into<String>, value: Value) -> Self {
        self.handler_config
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Set input JSON schema.
    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Set output JSON schema.
    pub fn output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Set execution mode.
    pub fn mode(mut self, m: impl Into<String>) -> Self {
        self.mode = Some(m.into());
        self
    }

    /// Set risk level ("low", "medium", "high").
    pub fn risk_level(mut self, level: impl Into<String>) -> Self {
        self.risk_level = Some(level.into());
        self
    }

    /// Set idempotency key template.
    pub fn idempotency_key(mut self, k: impl Into<String>) -> Self {
        self.idempotency_key = Some(k.into());
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

    /// Build the [`ActionType`].
    pub fn build(self) -> ActionType {
        ActionType {
            api_name: self.api_name,
            display: self.display,
            description: self.description,
            subject: self.subject,
            handler: ActionHandler {
                kind: self.handler_kind,
                target: self.handler_target,
                config: self.handler_config,
            },
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            mode: self.mode,
            risk_level: self.risk_level,
            idempotency_key: self.idempotency_key,
            deprecated_at: self.deprecated_at,
            metadata: self.metadata,
        }
    }
}

// ---------------------------------------------------------------------------
// AgentBuilder
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Operational primitive builders
// ---------------------------------------------------------------------------

/// Builder for [`ArtifactType`].
pub struct ArtifactTypeBuilder {
    api_name: ApiName,
    display: Option<String>,
    description: Option<String>,
    store: ApiName,
    path_template: String,
    media_type: Option<String>,
    metadata_schema: Vec<Property>,
    lifecycle: Vec<String>,
    metadata: Option<std::collections::BTreeMap<String, Value>>,
}

impl ArtifactTypeBuilder {
    /// Create a new artifact type builder.
    pub fn new(api_name: impl AsRef<str>, store: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            display: None,
            description: None,
            store: ApiName::new_unchecked(store.as_ref()),
            path_template: String::new(),
            media_type: None,
            metadata_schema: Vec::new(),
            lifecycle: Vec::new(),
            metadata: None,
        }
    }

    /// Set display name.
    pub fn display(mut self, v: impl Into<String>) -> Self {
        self.display = Some(v.into());
        self
    }

    /// Set description.
    pub fn description(mut self, v: impl Into<String>) -> Self {
        self.description = Some(v.into());
        self
    }

    /// Set path template.
    pub fn path_template(mut self, v: impl Into<String>) -> Self {
        self.path_template = v.into();
        self
    }

    /// Set media type.
    pub fn media_type(mut self, v: impl Into<String>) -> Self {
        self.media_type = Some(v.into());
        self
    }

    /// Add metadata property.
    pub fn metadata_property(mut self, p: Property) -> Self {
        self.metadata_schema.push(p);
        self
    }

    /// Add lifecycle state.
    pub fn state(mut self, v: impl Into<String>) -> Self {
        self.lifecycle.push(v.into());
        self
    }

    /// Add metadata key/value.
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Build the [`ArtifactType`].
    pub fn build(self) -> ArtifactType {
        ArtifactType {
            api_name: self.api_name,
            display: self.display,
            description: self.description,
            store: self.store,
            path_template: self.path_template,
            media_type: self.media_type,
            metadata_schema: self.metadata_schema,
            lifecycle: self.lifecycle,
            metadata: self.metadata,
        }
    }
}

/// Builder for [`UploadFlow`].
pub struct UploadFlowBuilder {
    api_name: ApiName,
    store: ApiName,
    accepted_formats: Vec<String>,
    max_bytes: Option<i64>,
    path_template: String,
    target_object_type: Option<ApiName>,
    mappings: Vec<tesela_ir::ColumnMapping>,
    quality_rules: Vec<QualityRule>,
    discover_schema: bool,
    rollback_required: bool,
    metadata: Option<std::collections::BTreeMap<String, Value>>,
}

impl UploadFlowBuilder {
    /// Create a new upload flow builder.
    pub fn new(api_name: impl AsRef<str>, store: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            store: ApiName::new_unchecked(store.as_ref()),
            accepted_formats: Vec::new(),
            max_bytes: None,
            path_template: String::new(),
            target_object_type: None,
            mappings: Vec::new(),
            quality_rules: Vec::new(),
            discover_schema: false,
            rollback_required: false,
            metadata: None,
        }
    }

    /// Add accepted formats.
    pub fn accepted_formats(mut self, formats: Vec<impl Into<String>>) -> Self {
        self.accepted_formats = formats.into_iter().map(Into::into).collect();
        self
    }

    /// Set maximum bytes.
    pub fn max_bytes(mut self, v: i64) -> Self {
        self.max_bytes = Some(v);
        self
    }

    /// Set path template.
    pub fn path_template(mut self, v: impl Into<String>) -> Self {
        self.path_template = v.into();
        self
    }

    /// Set target object type.
    pub fn target_object_type(mut self, v: impl AsRef<str>) -> Self {
        self.target_object_type = Some(ApiName::new_unchecked(v.as_ref()));
        self
    }

    /// Add a column mapping.
    pub fn mapping(
        mut self,
        source_column: impl Into<String>,
        target_property: impl AsRef<str>,
    ) -> Self {
        self.mappings.push(tesela_ir::ColumnMapping {
            source_column: source_column.into(),
            target_property: ApiName::new_unchecked(target_property.as_ref()),
            required: None,
            type_coercion: None,
            value_mapping: None,
        });
        self
    }

    /// Add a quality rule.
    pub fn quality_rule(mut self, rule: QualityRule) -> Self {
        self.quality_rules.push(rule);
        self
    }

    /// Enable schema discovery.
    pub fn discover_schema(mut self, enabled: bool) -> Self {
        self.discover_schema = enabled;
        self
    }

    /// Require rollback support.
    pub fn rollback_required(mut self, required: bool) -> Self {
        self.rollback_required = required;
        self
    }

    /// Add metadata key/value.
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Build the [`UploadFlow`].
    pub fn build(self) -> UploadFlow {
        UploadFlow {
            api_name: self.api_name,
            store: self.store,
            accepted_formats: self.accepted_formats,
            max_bytes: self.max_bytes,
            path_template: self.path_template,
            target_object_type: self.target_object_type,
            mappings: self.mappings,
            quality_rules: self.quality_rules,
            discover_schema: self.discover_schema,
            rollback_required: self.rollback_required,
            metadata: self.metadata,
        }
    }
}

/// Builder for [`JobType`].
pub struct JobTypeBuilder {
    api_name: ApiName,
    executor: ApiName,
    states: Vec<String>,
    idempotency_key: Option<String>,
    start_event: Option<ApiName>,
    result_event: Option<ApiName>,
    input_schema: Option<Value>,
    output_schema: Option<Value>,
}

impl JobTypeBuilder {
    /// Create a new job type builder.
    pub fn new(api_name: impl AsRef<str>, executor: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            executor: ApiName::new_unchecked(executor.as_ref()),
            states: Vec::new(),
            idempotency_key: None,
            start_event: None,
            result_event: None,
            input_schema: None,
            output_schema: None,
        }
    }

    /// Set lifecycle states.
    pub fn states(mut self, states: Vec<impl Into<String>>) -> Self {
        self.states = states.into_iter().map(Into::into).collect();
        self
    }

    /// Set idempotency key template.
    pub fn idempotency_key(mut self, v: impl Into<String>) -> Self {
        self.idempotency_key = Some(v.into());
        self
    }

    /// Set start event.
    pub fn start_event(mut self, v: impl AsRef<str>) -> Self {
        self.start_event = Some(ApiName::new_unchecked(v.as_ref()));
        self
    }

    /// Set result event.
    pub fn result_event(mut self, v: impl AsRef<str>) -> Self {
        self.result_event = Some(ApiName::new_unchecked(v.as_ref()));
        self
    }

    /// Set input schema.
    pub fn input_schema(mut self, schema: Value) -> Self {
        self.input_schema = Some(schema);
        self
    }

    /// Set output schema.
    pub fn output_schema(mut self, schema: Value) -> Self {
        self.output_schema = Some(schema);
        self
    }

    /// Build the [`JobType`].
    pub fn build(self) -> JobType {
        JobType {
            api_name: self.api_name,
            display: None,
            description: None,
            executor: self.executor,
            input_schema: self.input_schema,
            output_schema: self.output_schema,
            states: self.states,
            idempotency_key: self.idempotency_key,
            start_event: self.start_event,
            result_event: self.result_event,
            metadata: None,
        }
    }
}

/// Build a logical [`EventType`].
pub fn event_type(
    api_name: impl AsRef<str>,
    bus: impl AsRef<str>,
    topic: impl Into<String>,
) -> EventType {
    EventType {
        api_name: ApiName::new_unchecked(api_name.as_ref()),
        bus: ApiName::new_unchecked(bus.as_ref()),
        topic: topic.into(),
        payload_schema: None,
        correlation_keys: Vec::new(),
        metadata: None,
    }
}

/// Builder for [`CapabilityGrant`].
pub fn capability_grant(
    api_name: impl AsRef<str>,
    resource_kind: impl Into<String>,
    operations: Vec<Operation>,
) -> CapabilityGrant {
    CapabilityGrant {
        api_name: ApiName::new_unchecked(api_name.as_ref()),
        resource_kind: resource_kind.into(),
        resource: None,
        operations,
        ttl_seconds: None,
        constraints: None,
        metadata: None,
    }
}

/// Builder for a simple [`AggregateView`].
pub fn aggregate_view(
    api_name: impl AsRef<str>,
    object_type: impl AsRef<str>,
    measures: Vec<AggregateMeasure>,
) -> AggregateView {
    AggregateView {
        api_name: ApiName::new_unchecked(api_name.as_ref()),
        object_type: ApiName::new_unchecked(object_type.as_ref()),
        filter: None,
        group_by: Vec::new(),
        measures,
        time_bucket: None,
        spatial_extent: None,
        require_pushdown: true,
        metadata: None,
    }
}

/// Construct a typed aggregate measure.
pub fn measure(
    function: AggregateFunction,
    alias: impl Into<String>,
    property: Option<&str>,
) -> AggregateMeasure {
    AggregateMeasure {
        function,
        property: property.map(ApiName::new_unchecked),
        alias: alias.into(),
        distinct: false,
    }
}

/// Construct a time bucket.
pub fn time_bucket(property: impl AsRef<str>, interval: impl Into<String>) -> TimeBucket {
    TimeBucket {
        property: ApiName::new_unchecked(property.as_ref()),
        interval: interval.into(),
        timezone: None,
    }
}

/// Construct a spatial extent descriptor.
pub fn spatial_extent(property: impl AsRef<str>, output: impl Into<String>) -> SpatialExtent {
    SpatialExtent {
        property: ApiName::new_unchecked(property.as_ref()),
        output: output.into(),
    }
}

// ---------------------------------------------------------------------------
// PolicyBuilder
// ---------------------------------------------------------------------------

/// Builder for [`PolicyRule`].
pub struct PolicyBuilder {
    api_name: ApiName,
    effect: PolicyEffect,
    roles: Vec<String>,
    operations: Vec<Operation>,
    priority: Option<i32>,
    resource_kind: Option<String>,
    resource: Option<ApiName>,
    condition: Option<String>,
    row_filter: Option<tesela_ir::Filter>,
    redactions: Vec<ApiName>,
    obligations: Vec<tesela_ir::Obligation>,
}

impl PolicyBuilder {
    /// Create a new policy rule builder.
    pub fn new(api_name: impl AsRef<str>, effect: PolicyEffect) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            effect,
            roles: Vec::new(),
            operations: Vec::new(),
            priority: None,
            resource_kind: None,
            resource: None,
            condition: None,
            row_filter: None,
            redactions: Vec::new(),
            obligations: Vec::new(),
        }
    }

    /// Restrict to specific roles.
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Restrict to a specific operation.
    pub fn operation(mut self, op: Operation) -> Self {
        self.operations.push(op);
        self
    }

    /// Set evaluation priority.
    pub fn priority(mut self, p: i32) -> Self {
        self.priority = Some(p);
        self
    }

    /// Set resource kind.
    pub fn resource_kind(mut self, kind: impl Into<String>) -> Self {
        self.resource_kind = Some(kind.into());
        self
    }

    /// Set resource name.
    pub fn resource(mut self, name: impl AsRef<str>) -> Self {
        self.resource = Some(ApiName::new_unchecked(name.as_ref()));
        self
    }

    /// Set row filter.
    pub fn row_filter(mut self, filter: tesela_ir::Filter) -> Self {
        self.row_filter = Some(filter);
        self
    }

    /// Add a redaction field.
    pub fn redaction(mut self, field: impl AsRef<str>) -> Self {
        self.redactions.push(ApiName::new_unchecked(field.as_ref()));
        self
    }

    /// Build the [`PolicyRule`].
    pub fn build(self) -> PolicyRule {
        PolicyRule {
            api_name: self.api_name,
            effect: self.effect,
            actors: Vec::new(),
            roles: self.roles,
            operations: self.operations,
            resource_kind: self.resource_kind,
            resource: self.resource,
            condition: self.condition,
            row_filter: self.row_filter,
            redactions: self.redactions,
            obligations: self.obligations,
            priority: self.priority,
        }
    }
}

// ---------------------------------------------------------------------------
// DatasourceBuilder
// ---------------------------------------------------------------------------

/// Builder for [`Datasource`].
pub struct DatasourceBuilder {
    api_name: ApiName,
    adapter_type: String,
    config: Option<std::collections::BTreeMap<String, Value>>,
    secrets: Option<std::collections::BTreeMap<String, Value>>,
}

impl DatasourceBuilder {
    /// Create a new datasource builder.
    pub fn new(api_name: impl AsRef<str>, adapter_type: impl Into<String>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            adapter_type: adapter_type.into(),
            config: None,
            secrets: None,
        }
    }

    /// Add a config key/value.
    pub fn config(mut self, key: impl Into<String>, value: Value) -> Self {
        self.config
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Add a secret.
    pub fn secret(mut self, key: impl Into<String>, value: Value) -> Self {
        self.secrets
            .get_or_insert_with(std::collections::BTreeMap::new)
            .insert(key.into(), value);
        self
    }

    /// Build the [`Datasource`].
    pub fn build(self) -> Datasource {
        Datasource {
            api_name: self.api_name,
            adapter_type: self.adapter_type,
            config: self.config,
            secrets: self.secrets,
        }
    }
}

// ---------------------------------------------------------------------------
// RoleBuilder
// ---------------------------------------------------------------------------

/// Builder for [`tesela_ir::Role`].
pub struct RoleBuilder {
    api_name: ApiName,
    display: Option<String>,
    description: Option<String>,
    inherits: Vec<ApiName>,
}

impl RoleBuilder {
    /// Create a new role builder.
    pub fn new(api_name: impl AsRef<str>) -> Self {
        Self {
            api_name: ApiName::new_unchecked(api_name.as_ref()),
            display: None,
            description: None,
            inherits: Vec::new(),
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

    /// Add an inherited role.
    pub fn inherits(mut self, role: impl AsRef<str>) -> Self {
        self.inherits.push(ApiName::new_unchecked(role.as_ref()));
        self
    }

    /// Build the [`tesela_ir::Role`].
    pub fn build(self) -> tesela_ir::Role {
        tesela_ir::Role {
            api_name: self.api_name,
            display: self.display,
            description: self.description,
            inherits: self.inherits,
        }
    }
}

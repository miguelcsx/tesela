use super::prelude::*;

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

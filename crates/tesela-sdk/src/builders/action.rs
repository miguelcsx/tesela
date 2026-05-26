use super::prelude::*;

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

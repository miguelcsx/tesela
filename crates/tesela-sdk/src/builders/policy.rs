use super::prelude::*;

/// Builder for [`PolicyRule`].
pub struct PolicyBuilder {
    api_name: ApiName,
    description: Option<String>,
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
            description: None,
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

    /// Set description.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
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
            description: self.description,
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

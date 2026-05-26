use super::prelude::*;

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

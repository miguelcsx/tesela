use super::prelude::*;

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

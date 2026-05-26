use super::prelude::*;

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

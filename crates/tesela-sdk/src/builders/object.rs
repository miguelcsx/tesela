use super::prelude::*;

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

use super::prelude::*;

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

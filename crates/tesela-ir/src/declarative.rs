//! Declarative ontology descriptors.
//!
//! These descriptors are the authoring layer for Rust code. They keep ontology
//! definitions compact and static, then lower into the owned IR structs used for
//! serialization, validation, and runtime indexing.

use std::collections::BTreeMap;

use tesela_core::{ApiName, DataType, Value};

use crate::{Index, ObjectSource, ObjectType, Property};

/// Static property descriptor used by declarative object definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticProperty {
    /// Property API name.
    pub api_name: &'static str,
    /// Optional display label.
    pub display: Option<&'static str>,
    /// Optional description.
    pub description: Option<&'static str>,
    /// Property data type.
    pub data_type: DataType,
    /// Whether the property allows null values.
    pub nullable: bool,
    /// Whether the property should be indexed.
    pub indexed: bool,
    /// Whether the property must be unique.
    pub unique: bool,
    /// Optional source column mapping.
    pub source_column: Option<&'static str>,
    /// Whether the field should be encrypted by the platform store.
    pub encrypted: bool,
}

impl StaticProperty {
    /// Lower this descriptor into the owned IR property.
    #[must_use]
    pub fn to_ir(&self) -> Property {
        Property {
            api_name: ApiName::from(self.api_name),
            display: self.display.map(str::to_string),
            description: self.description.map(str::to_string),
            data_type: self.data_type,
            nullable: flag(self.nullable),
            indexed: flag(self.indexed),
            unique: flag(self.unique),
            tags: Vec::new(),
            markings: Vec::new(),
            default: None,
            source_column: self.source_column.map(str::to_string),
            allowed_values: None,
            sort_order: None,
            metadata: None,
            encrypted: flag(self.encrypted),
        }
    }
}

/// Static index descriptor used by declarative object definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticIndex {
    /// Index API name.
    pub api_name: &'static str,
    /// Indexed properties.
    pub properties: &'static [&'static str],
    /// Whether this index is unique.
    pub unique: bool,
}

impl StaticIndex {
    /// Lower this descriptor into the owned IR index.
    #[must_use]
    pub fn to_ir(&self) -> Index {
        Index {
            api_name: ApiName::from(self.api_name),
            properties: self
                .properties
                .iter()
                .map(|property| ApiName::from(*property))
                .collect(),
            unique: flag(self.unique),
        }
    }
}

/// Static object type descriptor used by declarative object definitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticObjectType {
    /// Object type API name.
    pub api_name: &'static str,
    /// Optional display label.
    pub display: Option<&'static str>,
    /// Optional description.
    pub description: Option<&'static str>,
    /// Datasource API name.
    pub datasource: &'static str,
    /// Optional physical resource name.
    pub resource: Option<&'static str>,
    /// Primary key property.
    pub primary_key: &'static str,
    /// Static property descriptors.
    pub properties: &'static [StaticProperty],
    /// Trait API names implemented by this object type.
    pub traits: &'static [&'static str],
    /// Tags for categorization.
    pub tags: &'static [&'static str],
    /// Index descriptors.
    pub indexes: &'static [StaticIndex],
}

impl StaticObjectType {
    /// Lower this descriptor into the owned IR object type.
    #[must_use]
    pub fn to_ir(&self) -> ObjectType {
        ObjectType {
            api_name: ApiName::from(self.api_name),
            display: self.display.map(str::to_string),
            description: self.description.map(str::to_string),
            source: ObjectSource {
                datasource: ApiName::from(self.datasource),
                resource: self.resource.map(str::to_string),
            },
            primary_key: ApiName::from(self.primary_key),
            properties: self.properties.iter().map(StaticProperty::to_ir).collect(),
            traits: self
                .traits
                .iter()
                .map(|value| ApiName::from(*value))
                .collect(),
            tags: self.tags.iter().map(|value| (*value).to_string()).collect(),
            metadata: None::<BTreeMap<String, Value>>,
            indexes: self.indexes.iter().map(StaticIndex::to_ir).collect(),
            deprecated_at: None,
        }
    }
}

/// Trait implemented by Rust types that declare a Tesela object type.
pub trait ObjectTypeDefinition {
    /// Return the static descriptor for this object type.
    fn definition() -> StaticObjectType;

    /// Lower this object definition into the owned IR.
    #[must_use]
    fn object_type() -> ObjectType {
        Self::definition().to_ir()
    }
}

fn flag(value: bool) -> Option<bool> {
    if value { Some(true) } else { None }
}

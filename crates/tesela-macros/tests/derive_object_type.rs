#![allow(dead_code)]

extern crate self as tesela;

pub mod core {
    pub use tesela_core::*;
}

pub mod ir {
    pub use tesela_ir::*;
}

#[derive(Clone, Copy)]
enum TestDatasource {
    Memory,
}

impl tesela::core::ApiNameSource for TestDatasource {
    fn api_name(&self) -> &'static str {
        match self {
            Self::Memory => "memory",
        }
    }
}

#[derive(Clone, Copy)]
enum CustomerField {
    Id,
}

impl tesela::core::ApiNameSource for CustomerField {
    fn api_name(&self) -> &'static str {
        match self {
            Self::Id => "id",
        }
    }
}

#[derive(tesela_macros::ObjectType)]
#[tesela(datasource = TestDatasource::Memory, primary_key = CustomerField::Id)]
struct Customer {
    #[tesela(indexed, unique, description = "Primary key")]
    id: String,
    #[tesela(nullable, encrypted)]
    email: String,
    #[tesela(indexed)]
    revenue: f64,
}

#[test]
fn test_object_type_derive() {
    let ot = Customer::tesela_object_type();
    assert_eq!(ot.api_name.as_ref(), "customer");
    assert_eq!(ot.source.datasource.as_ref(), "memory");
    assert_eq!(ot.properties.len(), 3);

    let id_prop = &ot.properties[0];
    assert_eq!(id_prop.api_name.as_ref(), "id");
    assert_eq!(id_prop.indexed, Some(true));
    assert_eq!(id_prop.unique, Some(true));

    let email_prop = &ot.properties[1];
    assert_eq!(email_prop.nullable, Some(true));
    assert_eq!(email_prop.encrypted, Some(true));

    let revenue_prop = &ot.properties[2];
    assert_eq!(revenue_prop.indexed, Some(true));
    assert_ne!(revenue_prop.nullable, Some(true));
}

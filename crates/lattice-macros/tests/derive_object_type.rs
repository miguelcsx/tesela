#![allow(dead_code)]

#[derive(lattice_macros::ObjectType)]
#[lattice(datasource = "memory", primary_key = "id")]
struct Customer {
    #[lattice(indexed, unique, description = "Primary key")]
    id: String,
    #[lattice(nullable, encrypted)]
    email: String,
    #[lattice(indexed)]
    revenue: f64,
}

#[derive(lattice_macros::Agent)]
#[lattice(model = "claude-sonnet-4-6")]
struct SupportAgent {}

#[test]
fn test_object_type_derive() {
    let ot = Customer::lattice_object_type();
    assert_eq!(ot.api_name.as_ref(), "customer");
    assert_eq!(ot.source.datasource.as_ref(), "memory");
    assert_eq!(ot.properties.len(), 3);

    let id_prop = &ot.properties[0];
    assert_eq!(id_prop.api_name.as_ref(), "id");
    assert!(id_prop.indexed.unwrap());
    assert!(id_prop.unique.unwrap());

    let email_prop = &ot.properties[1];
    assert!(email_prop.nullable.unwrap());
    assert!(email_prop.encrypted.unwrap());

    let revenue_prop = &ot.properties[2];
    assert!(revenue_prop.indexed.unwrap());
    assert!(!revenue_prop.nullable.unwrap_or(false));
}

#[test]
fn test_agent_derive() {
    let agent = SupportAgent::lattice_agent();
    assert_eq!(agent.api_name.as_ref(), "support_agent");
    assert_eq!(agent.model.as_deref(), Some("claude-sonnet-4-6"));
}

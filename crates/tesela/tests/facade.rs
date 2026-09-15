use tesela::{LinkType, ObjectType, TraitDef, action, policy};

#[derive(ObjectType)]
#[tesela(datasource = "memory", primary_key = "id")]
struct Customer {
    id: String,
    email: Option<String>,
}

#[derive(LinkType)]
#[tesela(from = "customer", to = "customer")]
struct Referral;

#[derive(TraitDef)]
struct Contact {
    email: String,
}

#[action(subject = "customer")]
fn welcome(email: String) -> String {
    email
}

#[policy(roles = "admin", operations = "execute")]
fn can_welcome() {}

#[test]
fn macros_work_with_facade_only() {
    let customer = Customer {
        id: "1".into(),
        email: None,
    };
    let contact = Contact {
        email: "a@example.com".into(),
    };
    assert_eq!(customer.id, "1");
    assert!(customer.email.is_none());
    assert_eq!(contact.email, "a@example.com");
    let object = Customer::tesela_object_type();
    assert_eq!(object.properties.len(), 2);
    assert_eq!(object.source.resource.as_deref(), Some("customer"));
    assert_eq!(object.properties[1].nullable, Some(true));
    let link = Referral::tesela_link_type();
    assert_eq!(link.from.as_ref(), "customer");
    assert_eq!(link.display.as_deref(), Some("Referral"));
    let trait_ = Contact::tesela_trait();
    assert_eq!(trait_.properties.len(), 1);
    assert_eq!(trait_.display.as_deref(), Some("Contact"));
    let action = WelcomeAction::tesela_action_type();
    assert_eq!(action.risk_level.as_deref(), Some("low"));
    assert_eq!(
        action.input_schema.unwrap().0["properties"]["email"]["type"],
        "string"
    );
    assert_eq!(action.subject.unwrap().as_ref(), "customer");
    assert_eq!(CanWelcomePolicy::tesela_policy_rule().roles, vec!["admin"]);
    assert_eq!(welcome("ok".to_string()), "ok");
    can_welcome();
}

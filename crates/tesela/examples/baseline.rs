use std::collections::BTreeMap;
use std::hint::black_box;
use std::sync::Arc;
use std::time::Instant;

use tesela::{
    Actor, AllowAllPolicy, ApiName, DataType, Datasource, Error, MemoryStore, Mutation,
    ObjectTypeDefinition, Query, Runtime, RuntimeOptions, Spec, StaticStoreRouter, Value,
};

#[derive(tesela::ObjectType)]
#[tesela(datasource = "memory", primary_key = "id")]
struct Customer {
    id: String,
    score: i64,
}

fn measure<T, F>(name: &str, count: usize, mut work: F) -> Result<(), Error>
where
    F: FnMut() -> Result<T, Error>,
{
    let start = Instant::now();
    for _ in 0..count {
        black_box(work()?);
    }
    println!(
        "{name}: {:.3} us/op ({count} ops)",
        start.elapsed().as_secs_f64() * 1_000_000.0 / count as f64
    );
    Ok(())
}

fn main() -> Result<(), Error> {
    let row = Customer {
        id: "0".into(),
        score: 0,
    };
    black_box((&row.id, row.score, DataType::String));
    let mut spec = Spec::default();
    spec.datasources.push(Datasource {
        api_name: ApiName::new("memory")?,
        adapter_type: "memory".into(),
        config: None,
        secrets: None,
    });
    spec.object_types.push(Customer::object_type());
    let json = spec.to_json_string()?;
    let store = MemoryStore::new();
    store.set_spec(spec.clone())?;
    let router = Arc::new(StaticStoreRouter::new());
    router.register(ApiName::new("memory")?, store)?;
    let runtime = Runtime::new(
        spec.clone(),
        RuntimeOptions {
            store_router: Some(router),
            policy_engine: Some(Arc::new(AllowAllPolicy)),
            ..RuntimeOptions::default()
        },
    )?;
    let actor = Actor {
        user_id: "bench".into(),
        roles: vec![],
        claims: BTreeMap::new(),
    };
    let object = ApiName::new("customer")?;
    for id in 0..100 {
        let mut values = BTreeMap::new();
        values.insert(ApiName::new("id")?, Value::string(id.to_string()));
        values.insert(ApiName::new("score")?, Value::integer(id));
        runtime.mutate(&actor, &object, Mutation::Create { values })?;
    }
    let mut update = BTreeMap::new();
    update.insert(ApiName::new("id")?, Value::string("0"));
    update.insert(ApiName::new("score")?, Value::integer(1));
    measure("spec.parse", 10_000, || Spec::parse(json.as_bytes()))?;
    measure("spec.serialize", 10_000, || spec.to_json_string())?;
    measure("search.100", 10_000, || {
        runtime.search(&actor, &object, Query::default())
    })?;
    measure("get", 10_000, || {
        runtime.get(&actor, &object, &Value::string("0"))
    })?;
    measure("mutate.upsert", 10_000, || {
        runtime.mutate(
            &actor,
            &object,
            Mutation::Upsert {
                values: update.clone(),
            },
        )
    })?;
    Ok(())
}

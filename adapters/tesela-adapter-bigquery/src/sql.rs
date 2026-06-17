use std::collections::BTreeMap;

use tesela_core::{ApiName, Error, FilterOp, Value};
use tesela_ir::Filter;
use tesela_runtime::query::{AggregateQuery, Query};

use crate::QueryParam;

pub(crate) fn search_sql(
    project_id: &str,
    dataset: &str,
    object_type: &ApiName,
    query: &Query,
) -> Result<(String, Vec<QueryParam>), Error> {
    let mut params = Vec::new();
    let mut sql = format!("SELECT * FROM {}", table(project_id, dataset, object_type)?);
    if let Some(filter) = &query.filter {
        let where_sql = compile_filter(filter, &mut params)?;
        sql.push_str(" WHERE ");
        sql.push_str(&where_sql);
    }
    if !query.sort.is_empty() {
        let order = query
            .sort
            .iter()
            .map(|sort| {
                let direction = match sort.direction.to_ascii_lowercase().as_str() {
                    "desc" => "DESC",
                    _ => "ASC",
                };
                Ok(format!("{} {direction}", field(&sort.property)?))
            })
            .collect::<Result<Vec<_>, Error>>()?
            .join(", ");
        sql.push_str(" ORDER BY ");
        sql.push_str(&order);
    }
    sql.push_str(&format!(
        " LIMIT {}",
        query.limit.unwrap_or(1000).clamp(1, 10_000)
    ));
    if let Some(offset) = query.offset {
        sql.push_str(&format!(" OFFSET {}", offset.max(0)));
    }
    Ok((sql, params))
}

pub(crate) fn aggregate_sql(
    project_id: &str,
    dataset: &str,
    object_type: &ApiName,
    query: &AggregateQuery,
) -> Result<(String, Vec<QueryParam>), Error> {
    let mut params = Vec::new();
    let mut select = query
        .group_by
        .iter()
        .map(field)
        .collect::<Result<Vec<_>, Error>>()?;
    for aggregation in &query.aggregations {
        let function = aggregation.function.to_ascii_lowercase();
        let expr = match function.as_str() {
            "count" => "COUNT(*)".to_string(),
            "sum" | "avg" | "min" | "max" => {
                let property = aggregation.property.as_ref().ok_or_else(|| {
                    Error::bad_request(format!("{} aggregation requires property", function))
                })?;
                format!("{}({})", function.to_ascii_uppercase(), field(property)?)
            }
            other => return Err(Error::unsupported(format!("bigquery aggregation {other}"))),
        };
        select.push(format!(
            "{expr} AS {}",
            field(&ApiName::new_unchecked(&aggregation.alias))?
        ));
    }
    if select.is_empty() {
        return Err(Error::bad_request(
            "aggregate query requires group_by or aggregations",
        ));
    }
    let mut sql = format!(
        "SELECT {} FROM {}",
        select.join(", "),
        table(project_id, dataset, object_type)?
    );
    if let Some(filter) = &query.filter {
        let where_sql = compile_filter(filter, &mut params)?;
        sql.push_str(" WHERE ");
        sql.push_str(&where_sql);
    }
    if !query.group_by.is_empty() {
        sql.push_str(" GROUP BY ");
        sql.push_str(
            &query
                .group_by
                .iter()
                .map(field)
                .collect::<Result<Vec<_>, Error>>()?
                .join(", "),
        );
    }
    sql.push_str(" LIMIT 10000");
    Ok((sql, params))
}

pub(crate) fn get_sql(
    project_id: &str,
    dataset: &str,
    object_type: &ApiName,
    pk: &Value,
) -> Result<(String, Vec<QueryParam>), Error> {
    Ok((
        format!(
            "SELECT * FROM {} WHERE `id` = @pk LIMIT 1",
            table(project_id, dataset, object_type)?
        ),
        vec![QueryParam::new("pk".to_string(), pk)],
    ))
}

pub(crate) fn insert_sql(
    project_id: &str,
    dataset: &str,
    object_type: &ApiName,
    values: &BTreeMap<ApiName, Value>,
) -> Result<(String, Vec<QueryParam>), Error> {
    require_values(values)?;
    let columns = values
        .keys()
        .map(field)
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let mut params = Vec::new();
    let placeholders = values
        .iter()
        .enumerate()
        .map(|(index, (_, value))| {
            let name = format!("p{index}");
            params.push(QueryParam::new(name.clone(), value));
            format!("@{name}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    Ok((
        format!(
            "INSERT INTO {} ({columns}) VALUES ({placeholders})",
            table(project_id, dataset, object_type)?
        ),
        params,
    ))
}

pub(crate) fn update_sql(
    project_id: &str,
    dataset: &str,
    object_type: &ApiName,
    primary_key: &Value,
    values: &BTreeMap<ApiName, Value>,
) -> Result<(String, Vec<QueryParam>), Error> {
    require_values(values)?;
    let mut params = vec![QueryParam::new("pk".to_string(), primary_key)];
    let assignments = values
        .iter()
        .enumerate()
        .map(|(index, (key, value))| {
            let name = format!("p{index}");
            params.push(QueryParam::new(name.clone(), value));
            Ok(format!("{} = @{name}", field(key)?))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    Ok((
        format!(
            "UPDATE {} SET {assignments} WHERE `id` = @pk",
            table(project_id, dataset, object_type)?
        ),
        params,
    ))
}

pub(crate) fn delete_sql(
    project_id: &str,
    dataset: &str,
    object_type: &ApiName,
    primary_key: &Value,
) -> Result<(String, Vec<QueryParam>), Error> {
    Ok((
        format!(
            "DELETE FROM {} WHERE `id` = @pk",
            table(project_id, dataset, object_type)?
        ),
        vec![QueryParam::new("pk".to_string(), primary_key)],
    ))
}

pub(crate) fn upsert_sql(
    project_id: &str,
    dataset: &str,
    object_type: &ApiName,
    values: &BTreeMap<ApiName, Value>,
) -> Result<(String, Vec<QueryParam>), Error> {
    require_values(values)?;
    if !values.contains_key(&ApiName::new_unchecked("id")) {
        return Err(Error::bad_request("bigquery upsert requires id"));
    }
    let mut params = Vec::new();
    let source = values
        .iter()
        .enumerate()
        .map(|(index, (key, value))| {
            let name = format!("p{index}");
            params.push(QueryParam::new(name.clone(), value));
            Ok(format!("@{name} AS {}", field(key)?))
        })
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let updates = values
        .keys()
        .filter(|key| key.to_string() != "id")
        .map(|key| Ok(format!("T.{0} = S.{0}", field(key)?)))
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let columns = values
        .keys()
        .map(field)
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let inserts = values
        .keys()
        .map(|key| Ok(format!("S.{}", field(key)?)))
        .collect::<Result<Vec<_>, Error>>()?
        .join(", ");
    let update_clause = if updates.is_empty() {
        String::new()
    } else {
        format!(" WHEN MATCHED THEN UPDATE SET {updates}")
    };
    Ok((
        format!(
            "MERGE {} T USING (SELECT {source}) S ON T.`id` = S.`id`{update_clause} WHEN NOT MATCHED THEN INSERT ({columns}) VALUES ({inserts})",
            table(project_id, dataset, object_type)?
        ),
        params,
    ))
}

fn compile_filter(filter: &Filter, params: &mut Vec<QueryParam>) -> Result<String, Error> {
    match filter.op {
        FilterOp::And | FilterOp::Or => {
            if filter.args.is_empty() {
                return Err(Error::bad_request("logical filter requires arguments"));
            }
            let joiner = if filter.op == FilterOp::And {
                " AND "
            } else {
                " OR "
            };
            let parts = filter
                .args
                .iter()
                .map(|filter| compile_filter(filter, params))
                .collect::<Result<Vec<_>, Error>>()?;
            Ok(format!("({})", parts.join(joiner)))
        }
        FilterOp::Not => {
            let Some(first) = filter.args.first() else {
                return Err(Error::bad_request("not filter requires one argument"));
            };
            Ok(format!("NOT ({})", compile_filter(first, params)?))
        }
        FilterOp::IsNull | FilterOp::IsNotNull => {
            let field = filter_field(filter)?;
            Ok(format!(
                "{} IS {}NULL",
                field,
                if filter.op == FilterOp::IsNotNull {
                    "NOT "
                } else {
                    ""
                }
            ))
        }
        FilterOp::In | FilterOp::NotIn => {
            if filter.values.is_empty() {
                return Err(Error::bad_request("in filter requires values"));
            }
            let names = filter
                .values
                .iter()
                .map(|value| push_param(params, value))
                .collect::<Vec<_>>()
                .join(", ");
            let op = if filter.op == FilterOp::In {
                "IN"
            } else {
                "NOT IN"
            };
            Ok(format!("{} {op} ({names})", filter_field(filter)?))
        }
        FilterOp::Between => {
            if filter.values.len() != 2 {
                return Err(Error::bad_request("between filter requires two values"));
            }
            Ok(format!(
                "{} BETWEEN {} AND {}",
                filter_field(filter)?,
                push_param(params, &filter.values[0]),
                push_param(params, &filter.values[1])
            ))
        }
        FilterOp::Contains | FilterOp::StartsWith | FilterOp::Like => {
            let value = filter_value(filter)?;
            let param = push_param(params, value);
            match filter.op {
                FilterOp::Contains => Ok(format!(
                    "STRPOS(CAST({} AS STRING), {param}) > 0",
                    filter_field(filter)?
                )),
                FilterOp::StartsWith => Ok(format!(
                    "STARTS_WITH(CAST({} AS STRING), {param})",
                    filter_field(filter)?
                )),
                FilterOp::Like => Ok(format!(
                    "CAST({} AS STRING) LIKE {param}",
                    filter_field(filter)?
                )),
                _ => unreachable!(),
            }
        }
        _ => {
            let op = match filter.op {
                FilterOp::Eq => "=",
                FilterOp::Ne => "!=",
                FilterOp::Lt => "<",
                FilterOp::Gt => ">",
                FilterOp::Lte => "<=",
                FilterOp::Gte => ">=",
                _ => {
                    return Err(Error::unsupported(format!(
                        "bigquery filter {:?}",
                        filter.op
                    )));
                }
            };
            Ok(format!(
                "{} {op} {}",
                filter_field(filter)?,
                push_param(params, filter_value(filter)?)
            ))
        }
    }
}

fn filter_field(filter: &Filter) -> Result<String, Error> {
    filter
        .field
        .as_ref()
        .ok_or_else(|| Error::bad_request("filter field is required"))
        .and_then(field)
}

fn filter_value(filter: &Filter) -> Result<&Value, Error> {
    filter
        .value
        .as_ref()
        .ok_or_else(|| Error::bad_request("filter value is required"))
}

fn push_param(params: &mut Vec<QueryParam>, value: &Value) -> String {
    let name = format!("p{}", params.len());
    params.push(QueryParam::new(name.clone(), value));
    format!("@{name}")
}

fn table(project_id: &str, dataset: &str, object_type: &ApiName) -> Result<String, Error> {
    Ok(format!(
        "`{}.{}.{}`",
        segment(project_id)?,
        segment(dataset)?,
        segment(&object_type.to_string())?
    ))
}

fn field(value: &ApiName) -> Result<String, Error> {
    Ok(format!("`{}`", segment(&value.to_string())?))
}

fn segment(value: &str) -> Result<&str, Error> {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
    {
        return Ok(value);
    }
    Err(Error::bad_request(format!(
        "invalid bigquery identifier: {value}"
    )))
}

fn require_values(values: &BTreeMap<ApiName, Value>) -> Result<(), Error> {
    if values.is_empty() {
        return Err(Error::bad_request("mutation values are required"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use tesela_core::{FilterOp, Value};
    use tesela_ir::Filter;
    use tesela_runtime::query::Query;

    use super::*;

    #[test]
    fn search_uses_parameters_for_filters() {
        let query = Query::default().and_filter(Filter::eq(
            ApiName::new_unchecked("organization_id"),
            Value::string("default"),
        ));
        let (sql, params) = search_sql(
            "project",
            "dataset",
            &ApiName::new_unchecked("widget"),
            &query,
        )
        .unwrap();

        assert_eq!(params.len(), 1);
        assert!(sql.contains("WHERE `organization_id` = @p0"));
        assert!(sql.contains("LIMIT 1000"));
    }

    #[test]
    fn upsert_generates_merge() {
        let mut values = BTreeMap::new();
        values.insert(ApiName::new_unchecked("id"), Value::string("1"));
        values.insert(ApiName::new_unchecked("name"), Value::string("A"));

        let (sql, params) = upsert_sql(
            "project",
            "dataset",
            &ApiName::new_unchecked("scenario"),
            &values,
        )
        .unwrap();

        assert_eq!(params.len(), 2);
        assert!(sql.starts_with("MERGE `project.dataset.scenario` T"));
        assert!(sql.contains("WHEN MATCHED THEN UPDATE SET"));
        assert!(sql.contains("WHEN NOT MATCHED THEN INSERT"));
    }

    #[test]
    fn aggregate_groups_and_counts() {
        let query = AggregateQuery {
            group_by: vec![ApiName::new_unchecked("mode")],
            aggregations: vec![tesela_runtime::query::Aggregation {
                function: "count".to_string(),
                property: None,
                alias: "trips".to_string(),
            }],
            ..AggregateQuery::default()
        };
        let (sql, params) = aggregate_sql(
            "project",
            "dataset",
            &ApiName::new_unchecked("trips"),
            &query,
        )
        .unwrap();

        assert!(params.is_empty());
        assert!(sql.contains("SELECT `mode`, COUNT(*) AS `trips`"));
        assert!(sql.contains("GROUP BY `mode`"));
    }

    #[test]
    fn rejects_empty_logical_filters() {
        let query = Query::default().and_filter(Filter {
            op: FilterOp::And,
            field: None,
            value: None,
            values: Vec::new(),
            args: Vec::new(),
        });

        let error = search_sql(
            "project",
            "dataset",
            &ApiName::new_unchecked("scenario"),
            &query,
        )
        .unwrap_err();

        assert!(
            error
                .to_string()
                .contains("logical filter requires arguments")
        );
    }

    #[test]
    fn rejects_empty_in_filters() {
        let query = Query::default().and_filter(Filter {
            field: Some(ApiName::new_unchecked("id")),
            op: FilterOp::In,
            value: None,
            values: Vec::new(),
            args: Vec::new(),
        });

        let error = search_sql(
            "project",
            "dataset",
            &ApiName::new_unchecked("scenario"),
            &query,
        )
        .unwrap_err();

        assert!(error.to_string().contains("in filter requires values"));
    }
}

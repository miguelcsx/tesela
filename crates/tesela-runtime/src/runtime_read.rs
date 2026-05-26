//! Read operations on the runtime (search, get, traverse, aggregate, explain).

use crate::query::*;
use crate::runtime::Runtime;
use crate::runtime_internal::apply_redactions;
use std::collections::BTreeMap;
use tesela_core::{ApiName, Error, Operation, Value};
use tesela_ir::{AggregateResult, ExplainPlan, Page, Record};

impl Runtime {
    /// Search records of an object type.
    #[tracing::instrument(skip(self, actor, query), err)]
    pub fn search(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        mut query: Query,
    ) -> Result<Page, Error> {
        let start = std::time::Instant::now();
        self.check_rate_limit(actor, "search")?;
        let decision =
            self.authorize_with_decision(actor, Operation::Search, "object_type", object_name)?;

        let snap = self.ontology()?;
        let ot = snap
            .object_types
            .get(object_name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_name))?;

        if query.limit.unwrap_or(self.max_query_limit) > self.max_query_limit {
            query.limit = Some(self.max_query_limit);
        }

        if let Some(ref rf) = decision.row_filter {
            query = query.and_filter(rf.clone());
        }

        let backend = self.acquire_backend(&ot.source.datasource)?;
        let searcher = backend
            .as_ref()
            .as_searcher()
            .ok_or_else(|| Error::unsupported("search"))?;
        let mut page = searcher.search(object_name, &query)?;

        if let Some(sealer) = &self.sealer {
            for record in &mut page.records {
                let _ = crate::encrypt::decrypt_record_fields(record, &ot, sealer.as_ref());
            }
        }

        if !decision.redactions.is_empty() {
            for record in &mut page.records {
                apply_redactions(record, &decision.redactions);
            }
        }

        for record in &mut page.records {
            crate::computed::materialize_computed(record, &ot, self.computed_evaluator.as_deref());
        }

        self.run_obligations(actor, &decision, &BTreeMap::new())?;

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        self.metric_counter("tesela_search_total", "Total search calls", &[])
            .inc(1);
        self.metric_histogram("tesela_search_duration_ms", "Search latency", &[])
            .record(elapsed_ms);

        self.audit_and_event(
            actor,
            Operation::Search,
            "object_type",
            object_name,
            true,
            page.records.len() as i64,
        )?;
        Ok(page)
    }

    /// Get a single record by primary key.
    #[tracing::instrument(skip(self, actor, pk), err)]
    pub fn get(&self, actor: &Actor, object_name: &ApiName, pk: &Value) -> Result<Record, Error> {
        self.check_rate_limit(actor, "get")?;
        let decision =
            self.authorize_with_decision(actor, Operation::Read, "object_type", object_name)?;

        let snap = self.ontology()?;
        let ot = snap
            .object_types
            .get(object_name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_name))?;

        let backend = self.acquire_backend(&ot.source.datasource)?;
        let getter = backend
            .as_ref()
            .as_getter()
            .ok_or_else(|| Error::unsupported("get"))?;
        let mut record = getter
            .get(object_name, pk)?
            .ok_or_else(|| Error::not_found("record", pk))?;

        if let Some(sealer) = &self.sealer {
            let _ = crate::encrypt::decrypt_record_fields(&mut record, &ot, sealer.as_ref());
        }

        if !decision.redactions.is_empty() {
            apply_redactions(&mut record, &decision.redactions);
        }

        crate::computed::materialize_computed(&mut record, &ot, self.computed_evaluator.as_deref());

        self.run_obligations(actor, &decision, &BTreeMap::new())?;

        self.audit_and_event(actor, Operation::Read, "object_type", object_name, true, 1)?;
        Ok(record)
    }

    /// Traverse a link from a source record.
    #[tracing::instrument(skip(self, actor, query), err)]
    pub fn traverse(
        &self,
        actor: &Actor,
        link_name: &ApiName,
        query: TraversalQuery,
    ) -> Result<Page, Error> {
        let decision =
            self.authorize_with_decision(actor, Operation::Traverse, "link_type", link_name)?;

        let snap = self.ontology()?;
        let link = snap
            .links
            .get(link_name)
            .cloned()
            .ok_or_else(|| Error::not_found("link_type", link_name))?;

        let ds_name = link
            .source
            .as_ref()
            .and_then(|s| s.datasource.as_ref())
            .cloned()
            .unwrap_or_else(|| ApiName::new_unchecked("memory"));
        let backend = self.acquire_backend(&ds_name)?;
        let traverser = backend
            .as_ref()
            .as_traverser()
            .ok_or_else(|| Error::unsupported("traverse"))?;
        let mut page = traverser.traverse(link_name, &query)?;

        if !decision.redactions.is_empty() {
            for record in &mut page.records {
                apply_redactions(record, &decision.redactions);
            }
        }

        self.run_obligations(actor, &decision, &BTreeMap::new())?;

        self.audit_and_event(
            actor,
            Operation::Traverse,
            "link_type",
            link_name,
            true,
            page.records.len() as i64,
        )?;
        Ok(page)
    }

    /// Aggregate records of an object type.
    #[tracing::instrument(skip(self, actor, query), err)]
    pub fn aggregate(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        mut query: AggregateQuery,
    ) -> Result<AggregateResult, Error> {
        self.authorize_with_decision(actor, Operation::Aggregate, "object_type", object_name)?;

        let snap = self.ontology()?;
        let ot = snap
            .object_types
            .get(object_name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_name))?;

        let backend = self.acquire_backend(&ot.source.datasource)?;
        let aggregator = backend
            .as_ref()
            .as_aggregator()
            .ok_or_else(|| Error::unsupported("aggregate"))?;

        if let Some(planner) = &self.query_planner {
            let plan = planner.plan_aggregate(&ot, &query)?;
            if query.require_pushdown && !plan.push_down {
                return Err(Error::unsupported(
                    "aggregate query requires pushdown but planner selected fallback",
                ));
            }
            if !plan.push_down {
                query = plan.fallback_query;
            }
        } else if query.require_pushdown
            || query.time_bucket.is_some()
            || query.spatial_extent.is_some()
        {
            return Err(Error::unsupported(
                "aggregate query requires planner support for pushdown, time buckets, or spatial extent",
            ));
        }

        let result = aggregator.aggregate(object_name, &query)?;

        self.audit_and_event(
            actor,
            Operation::Aggregate,
            "object_type",
            object_name,
            true,
            result.groups.len() as i64,
        )?;
        Ok(result)
    }

    /// Explain a search query.
    #[tracing::instrument(skip(self, actor, query), err)]
    pub fn explain(
        &self,
        actor: &Actor,
        object_name: &ApiName,
        query: Query,
    ) -> Result<ExplainPlan, Error> {
        self.authorize_with_decision(actor, Operation::Search, "object_type", object_name)?;
        let snap = self.ontology()?;
        let ot = snap
            .object_types
            .get(object_name)
            .cloned()
            .ok_or_else(|| Error::not_found("object_type", object_name))?;

        let backend = self.acquire_backend(&ot.source.datasource)?;
        let explainer = backend
            .as_ref()
            .as_explainer()
            .ok_or_else(|| Error::unsupported("explain"))?;
        let plan = explainer.explain_search(object_name, &query)?;

        self.audit_and_event(
            actor,
            Operation::Search,
            "object_type",
            object_name,
            true,
            0,
        )?;
        Ok(plan)
    }
}

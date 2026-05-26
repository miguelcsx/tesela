//! Dynamic pipeline DAG execution.
//!
//! Unlike a static DAG executor, `DynamicPipelineExecutor` maintains a mutable
//! `ExecutionQueue` that steps can modify during execution — injecting new steps,
//! removing pending steps, or rerouting sources.

use crate::ports::{BackendRegistry, PipelineExecutor};
use crate::query::{Mutation, Query};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::Instant;
use tesela_core::{ApiName, Error, Value};
use tesela_ir::{
    ErrorStrategy, ExecutionMode, PipelineContext, PipelineResult, RouteChange, StepDirective,
    StepKind, StepResult, StepStatus, TransformPipeline, TransformStep,
};

/// Evaluates boolean expressions and source references against pipeline context.
pub trait PipelineConditionEvaluator: Send + Sync {
    /// Evaluate a boolean expression. Returns `true` if the step should execute.
    fn evaluate(&self, expr: &str, ctx: &PipelineContext) -> Result<bool, Error>;
    /// Resolve a dynamic source expression to an api_name.
    fn resolve_source(&self, expr: &str, ctx: &PipelineContext) -> Result<ApiName, Error>;
    /// Evaluate a decision expression, returning a directive for DAG mutation.
    fn evaluate_decision(&self, expr: &str, ctx: &PipelineContext) -> Result<StepDirective, Error>;
}

/// Simple built-in expression evaluator.
///
/// Supports: `"true"`, `"false"`, `var == 'value'`, `var != 'value'`.
/// For decision expressions, returns an empty directive (no-op) — users should
/// register a custom evaluator for real decision logic.
pub struct SimplePipelineConditionEvaluator;

impl PipelineConditionEvaluator for SimplePipelineConditionEvaluator {
    fn evaluate(&self, expr: &str, ctx: &PipelineContext) -> Result<bool, Error> {
        let expr = expr.trim();
        if expr.eq_ignore_ascii_case("true") {
            return Ok(true);
        }
        if expr.eq_ignore_ascii_case("false") {
            return Ok(false);
        }
        if let Some((var, val)) = parse_equality(expr) {
            let negated = expr.contains("!=");
            let actual = ctx.variables.get(var).or_else(|| ctx.metadata.get(var));
            let matches = match actual {
                Some(v) if v.as_str().is_some() => v.as_str().unwrap() == val,
                Some(other) => other.to_string().trim_matches('"') == val,
                None => val.is_empty(),
            };
            return Ok(if negated { !matches } else { matches });
        }
        // Check if variable is truthy
        if let Some(v) = ctx.variables.get(expr).or_else(|| ctx.metadata.get(expr)) {
            return Ok(is_truthy(v));
        }
        Ok(false)
    }

    fn resolve_source(&self, expr: &str, ctx: &PipelineContext) -> Result<ApiName, Error> {
        let expr = expr.trim();
        // Direct variable lookup
        if let Some(v) = ctx.variables.get(expr).or_else(|| ctx.metadata.get(expr))
            && let Some(s) = v.as_str()
        {
            return ApiName::new(s);
        }
        // Treat expression as a literal api_name
        ApiName::new(expr)
    }

    fn evaluate_decision(
        &self,
        _expr: &str,
        _ctx: &PipelineContext,
    ) -> Result<StepDirective, Error> {
        Ok(StepDirective::default())
    }
}

fn parse_equality(expr: &str) -> Option<(&str, &str)> {
    let (var, val) = if let Some(pos) = expr.find("!=") {
        (expr[..pos].trim(), expr[pos + 2..].trim())
    } else if let Some(pos) = expr.find("==") {
        (expr[..pos].trim(), expr[pos + 2..].trim())
    } else {
        return None;
    };
    let val = val
        .trim_start_matches('\'')
        .trim_end_matches('\'')
        .trim_start_matches('"')
        .trim_end_matches('"');
    Some((var, val))
}

fn is_truthy(v: &Value) -> bool {
    if v.is_null() {
        return false;
    }
    if let Some(b) = v.0.as_bool() {
        return b;
    }
    if let Some(n) = v.0.as_f64() {
        return n != 0.0;
    }
    if let Some(s) = v.as_str() {
        return !s.is_empty() && s != "false" && s != "0";
    }
    if let Some(a) = v.0.as_array() {
        return !a.is_empty();
    }
    if let Some(o) = v.0.as_object() {
        return !o.is_empty();
    }
    false
}

/// Mutable queue of steps that can be modified during pipeline execution.
struct ExecutionQueue {
    pending: VecDeque<TransformStep>,
    completed: Vec<ApiName>,
}

impl ExecutionQueue {
    fn from_steps(steps: Vec<TransformStep>) -> Self {
        let sorted = topo_sort_owned(steps);
        Self {
            pending: VecDeque::from(sorted),
            completed: Vec::new(),
        }
    }

    fn pop(&mut self) -> Option<TransformStep> {
        self.pending.pop_front()
    }

    fn mark_completed(&mut self, name: ApiName) {
        self.completed.push(name);
    }

    fn inject(&mut self, steps: Vec<TransformStep>) {
        for step in steps {
            self.pending.push_back(step);
        }
        // Re-sort the pending queue to respect new dependencies
        let drained: Vec<TransformStep> = self.pending.drain(..).collect();
        self.pending = VecDeque::from(topo_sort_owned(drained));
    }

    fn remove(&mut self, names: &[ApiName]) {
        self.pending.retain(|s| !names.contains(&s.api_name));
    }

    fn reroute(&mut self, changes: &[RouteChange]) {
        for change in changes {
            for step in self.pending.iter_mut() {
                if step.api_name == change.step {
                    step.source = change.new_source.clone();
                }
            }
        }
    }

    fn find_step(&self, name: &ApiName) -> Option<&TransformStep> {
        self.pending.iter().find(|s| &s.api_name == name)
    }
}

/// Topologically sort steps (owned version for the mutable queue).
fn topo_sort_owned(steps: Vec<TransformStep>) -> Vec<TransformStep> {
    let n = steps.len();
    if n == 0 {
        return steps;
    }
    let mut in_degree: Vec<usize> = vec![0; n];
    let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

    for (i, s) in steps.iter().enumerate() {
        for (j, t) in steps.iter().enumerate() {
            if i != j && t.target == s.source {
                adj[j].push(i);
                in_degree[i] += 1;
            }
        }
    }

    let mut queue: VecDeque<usize> = (0..n).filter(|&i| in_degree[i] == 0).collect();
    let mut order = Vec::with_capacity(n);

    while let Some(idx) = queue.pop_front() {
        order.push(idx);
        for &dep in &adj[idx] {
            in_degree[dep] -= 1;
            if in_degree[dep] == 0 {
                queue.push_back(dep);
            }
        }
    }

    // If some steps weren't reached (cycle), append them at the end
    for i in 0..n {
        if !order.contains(&i) {
            order.push(i);
        }
    }

    // Move steps out by index. Build a vec of Options so we can take by index.
    let mut slots: Vec<Option<TransformStep>> = steps.into_iter().map(Some).collect();
    order.into_iter().filter_map(|i| slots[i].take()).collect()
}

/// Dynamic pipeline executor that supports mid-execution DAG mutation.
pub struct DynamicPipelineExecutor {
    registry: Arc<dyn BackendRegistry>,
    condition_eval: Box<dyn PipelineConditionEvaluator>,
}

impl DynamicPipelineExecutor {
    /// Create an executor with the default simple condition evaluator.
    pub fn new(registry: Arc<dyn BackendRegistry>) -> Self {
        Self {
            registry,
            condition_eval: Box::new(SimplePipelineConditionEvaluator),
        }
    }

    /// Create an executor with a custom condition evaluator.
    pub fn with_evaluator(
        registry: Arc<dyn BackendRegistry>,
        evaluator: Box<dyn PipelineConditionEvaluator>,
    ) -> Self {
        Self {
            registry,
            condition_eval: evaluator,
        }
    }

    fn execute_transform_step(
        &self,
        step: &TransformStep,
        _mode: ExecutionMode,
        ctx: &PipelineContext,
    ) -> Result<i64, Error> {
        // Resolve source: dynamic_source overrides static source
        let source_name = if let Some(ref dyn_src) = step.dynamic_source {
            self.condition_eval.resolve_source(dyn_src, ctx)?
        } else {
            step.source.clone()
        };
        let target_name = step.target.clone();

        let src_backend = self.registry.acquire(&source_name)?;
        let searcher = src_backend.as_searcher().ok_or_else(|| {
            Error::unsupported(format!("source '{}' does not support search", source_name))
        })?;

        let page = searcher.search(&source_name, &Query::default())?;

        let tgt_backend = self.registry.acquire(&target_name)?;
        let mutator = tgt_backend.as_mutator().ok_or_else(|| {
            Error::unsupported(format!("target '{}' does not support mutate", target_name))
        })?;

        let mut written = 0i64;
        for record in page.records {
            let mutation = Mutation::Upsert {
                values: record.values,
            };
            match mutator.mutate(&target_name, &mutation) {
                Ok(r) => written += r.rows_affected.unwrap_or(1),
                Err(e) => return Err(e),
            }
        }
        Ok(written)
    }
}

impl PipelineExecutor for DynamicPipelineExecutor {
    fn execute(
        &self,
        pipeline: &TransformPipeline,
        mode: ExecutionMode,
    ) -> Result<PipelineResult, Error> {
        let started = Instant::now();
        let mut queue = ExecutionQueue::from_steps(pipeline.steps.clone());
        let mut ctx = PipelineContext {
            metadata: pipeline.context.clone().unwrap_or_default(),
            variables: BTreeMap::new(),
        };
        let mut total_written = 0i64;
        let mut errors = Vec::new();
        let mut step_results = Vec::new();

        while let Some(step) = queue.pop() {
            let step_name = step.api_name.clone();
            let kind = step.kind.unwrap_or_default();

            // Evaluate `when` condition
            if let Some(ref when_expr) = step.when {
                match self.condition_eval.evaluate(when_expr, &ctx) {
                    Ok(false) => {
                        step_results.push(StepResult {
                            step: step_name.clone(),
                            status: StepStatus::Skipped,
                            records_written: 0,
                            error: None,
                            injected_steps: Vec::new(),
                        });
                        queue.mark_completed(step_name);
                        continue;
                    }
                    Ok(true) => {}
                    Err(e) => {
                        errors.push(format!("step '{}': when eval error: {}", step_name, e));
                        step_results.push(StepResult {
                            step: step_name.clone(),
                            status: StepStatus::Failed,
                            records_written: 0,
                            error: Some(e.to_string()),
                            injected_steps: Vec::new(),
                        });
                        queue.mark_completed(step_name);
                        continue;
                    }
                }
            }

            match kind {
                StepKind::Transform => match self.execute_transform_step(&step, mode, &ctx) {
                    Ok(written) => {
                        total_written += written;
                        ctx.variables.insert(
                            format!("{}.records_written", step_name),
                            Value::from(written),
                        );
                        step_results.push(StepResult {
                            step: step_name.clone(),
                            status: StepStatus::Executed,
                            records_written: written,
                            error: None,
                            injected_steps: Vec::new(),
                        });
                    }
                    Err(e) => {
                        let err_msg = format!("step '{}': {}", step_name, e);
                        errors.push(err_msg.clone());

                        match step.on_error.as_ref() {
                            Some(ErrorStrategy::Abort) => {
                                step_results.push(StepResult {
                                    step: step_name.clone(),
                                    status: StepStatus::Failed,
                                    records_written: 0,
                                    error: Some(e.to_string()),
                                    injected_steps: Vec::new(),
                                });
                                break;
                            }
                            Some(ErrorStrategy::Fallback { step: fb_name }) => {
                                if let Some(fb_step) = queue.find_step(fb_name).cloned() {
                                    match self.execute_transform_step(&fb_step, mode, &ctx) {
                                        Ok(written) => {
                                            total_written += written;
                                            step_results.push(StepResult {
                                                step: step_name.clone(),
                                                status: StepStatus::Failed,
                                                records_written: 0,
                                                error: Some(e.to_string()),
                                                injected_steps: Vec::new(),
                                            });
                                            step_results.push(StepResult {
                                                step: fb_name.clone(),
                                                status: StepStatus::Executed,
                                                records_written: written,
                                                error: None,
                                                injected_steps: Vec::new(),
                                            });
                                            queue.remove(std::slice::from_ref(fb_name));
                                        }
                                        Err(fb_err) => {
                                            errors.push(format!(
                                                "step '{}': fallback also failed: {}",
                                                fb_name, fb_err
                                            ));
                                            step_results.push(StepResult {
                                                step: step_name.clone(),
                                                status: StepStatus::Failed,
                                                records_written: 0,
                                                error: Some(e.to_string()),
                                                injected_steps: Vec::new(),
                                            });
                                        }
                                    }
                                } else {
                                    step_results.push(StepResult {
                                        step: step_name.clone(),
                                        status: StepStatus::Failed,
                                        records_written: 0,
                                        error: Some(e.to_string()),
                                        injected_steps: Vec::new(),
                                    });
                                }
                            }
                            None | Some(ErrorStrategy::Skip) => {
                                step_results.push(StepResult {
                                    step: step_name.clone(),
                                    status: StepStatus::Failed,
                                    records_written: 0,
                                    error: Some(e.to_string()),
                                    injected_steps: Vec::new(),
                                });
                            }
                        }
                    }
                },

                StepKind::Decision => {
                    let expr = step.expression.as_deref().unwrap_or("true");
                    match self.condition_eval.evaluate_decision(expr, &ctx) {
                        Ok(directive) => {
                            let injected_names: Vec<ApiName> = directive
                                .inject
                                .iter()
                                .map(|s| s.api_name.clone())
                                .collect();

                            if !directive.remove.is_empty() {
                                queue.remove(&directive.remove);
                            }
                            if !directive.reroute.is_empty() {
                                queue.reroute(&directive.reroute);
                            }
                            if !directive.inject.is_empty() {
                                queue.inject(directive.inject);
                            }

                            step_results.push(StepResult {
                                step: step_name.clone(),
                                status: StepStatus::Executed,
                                records_written: 0,
                                error: None,
                                injected_steps: injected_names,
                            });
                        }
                        Err(e) => {
                            errors.push(format!("step '{}': decision error: {}", step_name, e));
                            step_results.push(StepResult {
                                step: step_name.clone(),
                                status: StepStatus::Failed,
                                records_written: 0,
                                error: Some(e.to_string()),
                                injected_steps: Vec::new(),
                            });
                        }
                    }
                }

                StepKind::Fork | StepKind::Join => {
                    // Fork/Join: sequential for now, just record as executed
                    step_results.push(StepResult {
                        step: step_name.clone(),
                        status: StepStatus::Executed,
                        records_written: 0,
                        error: None,
                        injected_steps: Vec::new(),
                    });
                }
            }

            queue.mark_completed(step_name);
        }

        Ok(PipelineResult {
            records_written: total_written,
            duration_ms: started.elapsed().as_millis() as u64,
            errors,
            step_results,
        })
    }
}

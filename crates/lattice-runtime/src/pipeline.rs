//! Transform pipeline DAG execution.

use crate::ports::{BackendRegistry, PipelineExecutor};
use crate::query::{Mutation, Query};
use lattice_core::Error;
use lattice_ir::{ExecutionMode, PipelineResult, TransformPipeline, TransformStep};
use std::sync::Arc;
use std::time::Instant;

/// Executes transform pipelines using the registered backends.
///
/// Steps are ordered topologically (sources before targets).  For each step
/// the executor reads all matching records from the source object type and
/// writes them to the target via the backend's `Mutator` port.
pub struct DefaultPipelineExecutor {
    registry: Arc<dyn BackendRegistry>,
}

impl DefaultPipelineExecutor {
    /// Create a new executor backed by the given registry.
    pub fn new(registry: Arc<dyn BackendRegistry>) -> Self {
        Self { registry }
    }

    /// Topologically sort steps by their source/target dependencies.
    fn topo_sort(steps: &[TransformStep]) -> Vec<&TransformStep> {
        // Build an adjacency list: step index → indices of steps that depend on it.
        let n = steps.len();
        let mut in_degree: Vec<usize> = vec![0; n];
        let mut adj: Vec<Vec<usize>> = vec![Vec::new(); n];

        // A step depends on another if the other's target is this step's source.
        for (i, s) in steps.iter().enumerate() {
            for (j, t) in steps.iter().enumerate() {
                if i != j && t.target == s.source {
                    adj[j].push(i);
                    in_degree[i] += 1;
                }
            }
        }

        // Kahn's algorithm.
        let mut queue: std::collections::VecDeque<usize> =
            (0..n).filter(|&i| in_degree[i] == 0).collect();
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

        order.iter().map(|&i| &steps[i]).collect()
    }

    /// Execute a single step: read source, transform (identity for now), write target.
    fn execute_step(
        &self,
        step: &TransformStep,
        _mode: ExecutionMode,
        errors: &mut Vec<String>,
    ) -> i64 {
        let source_ds = step.source.clone();
        let target_ds = step.target.clone();

        // Acquire source backend.
        let src_backend = match self.registry.acquire(&source_ds) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!(
                    "step '{}': source backend error: {}",
                    step.api_name, e
                ));
                return 0;
            }
        };
        let searcher = match src_backend.as_searcher() {
            Some(s) => s,
            None => {
                errors.push(format!(
                    "step '{}': source '{}' does not support search",
                    step.api_name, source_ds
                ));
                return 0;
            }
        };

        // Read all records from source.
        let page = match searcher.search(&source_ds, &Query::default()) {
            Ok(p) => p,
            Err(e) => {
                errors.push(format!("step '{}': search failed: {}", step.api_name, e));
                return 0;
            }
        };

        // Acquire target backend.
        let tgt_backend = match self.registry.acquire(&target_ds) {
            Ok(b) => b,
            Err(e) => {
                errors.push(format!(
                    "step '{}': target backend error: {}",
                    step.api_name, e
                ));
                return 0;
            }
        };
        let mutator = match tgt_backend.as_mutator() {
            Some(m) => m,
            None => {
                errors.push(format!(
                    "step '{}': target '{}' does not support mutate",
                    step.api_name, target_ds
                ));
                return 0;
            }
        };

        // Snapshot mode: not deleting existing records here (backend responsibility).
        let mut written = 0i64;
        for record in page.records {
            let mutation = Mutation::Upsert {
                values: record.values,
            };
            match mutator.mutate(&target_ds, &mutation) {
                Ok(r) => written += r.rows_affected.unwrap_or(1),
                Err(e) => {
                    errors.push(format!("step '{}': mutate failed: {}", step.api_name, e));
                }
            }
        }
        written
    }
}

impl PipelineExecutor for DefaultPipelineExecutor {
    fn execute(
        &self,
        pipeline: &TransformPipeline,
        mode: ExecutionMode,
    ) -> Result<PipelineResult, Error> {
        let started = Instant::now();
        let ordered = Self::topo_sort(&pipeline.steps);
        let mut total_written = 0i64;
        let mut errors = Vec::new();

        for step in ordered {
            total_written += self.execute_step(step, mode, &mut errors);
        }

        Ok(PipelineResult {
            records_written: total_written,
            duration_ms: started.elapsed().as_millis() as u64,
            errors,
        })
    }
}

//! Pass trait, compile result, and compiler pipeline.

use lattice_core::Diagnostic;
use lattice_graph::{GraphBuilder, SchemaGraph};
use lattice_ir::Spec;

use crate::hash::normalize_spec;
use crate::passes::*;

/// A single compilation pass that validates or transforms a spec.
pub trait Pass: Send + Sync {
    /// Human-readable pass name.
    fn name(&self) -> &'static str;

    /// Run the pass over the spec and graph.
    fn run(&self, spec: &Spec, graph: &SchemaGraph) -> Vec<Diagnostic>;
}

/// Result of a compilation run.
#[derive(Debug, Clone, PartialEq)]
pub struct CompileResult {
    /// All diagnostics emitted during compilation.
    pub diagnostics: Vec<Diagnostic>,
    /// Normalized spec (only present if compilation succeeded).
    pub spec: Option<Spec>,
    /// Whether compilation is valid (no error-level diagnostics).
    pub is_valid: bool,
}

impl CompileResult {
    /// Create a failed result with only diagnostics.
    pub fn failed(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics,
            spec: None,
            is_valid: false,
        }
    }

    /// Create a successful result with normalized spec.
    pub fn success(spec: Spec, diagnostics: Vec<Diagnostic>) -> Self {
        let is_valid = !diagnostics.iter().any(|d| d.is_error());
        Self {
            diagnostics,
            spec: Some(spec),
            is_valid,
        }
    }
}

/// Compiler pipeline that runs a sequence of passes over a spec.
#[derive(Default)]
pub struct Compiler {
    passes: Vec<Box<dyn Pass>>,
}

impl Compiler {
    /// Create a compiler with the default pass pipeline.
    pub fn default_pipeline() -> Self {
        let mut compiler = Self::new();
        compiler.add_pass(Box::new(NameValidationPass));
        compiler.add_pass(Box::new(ReferenceValidationPass));
        compiler.add_pass(Box::new(PropertyValidationPass));
        compiler.add_pass(Box::new(PolicyValidationPass));
        compiler.add_pass(Box::new(LinkValidationPass));
        compiler.add_pass(Box::new(NormalizationPass));
        compiler
    }

    /// Create an empty compiler.
    pub fn new() -> Self {
        Self { passes: Vec::new() }
    }

    /// Add a pass to the pipeline.
    pub fn add_pass(&mut self, pass: Box<dyn Pass>) {
        self.passes.push(pass);
    }

    /// Compile a spec: run all passes, normalize, return result.
    pub fn compile(&self, spec: &Spec) -> CompileResult {
        let graph = GraphBuilder::build(spec);
        let mut diagnostics = Vec::new();

        for pass in &self.passes {
            let pass_diags = pass.run(spec, &graph);
            diagnostics.extend(pass_diags);
        }

        if diagnostics.iter().any(|d| d.is_error()) {
            return CompileResult::failed(diagnostics);
        }

        let normalized = normalize_spec(spec.clone());
        CompileResult::success(normalized, diagnostics)
    }
}

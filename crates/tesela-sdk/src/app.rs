//! Top-level `App` builder for defining a complete Tesela ontology.

use tesela_compiler::{CompileResult, Compiler};
use tesela_core::{ApiName, Error};
use tesela_ir::{
    ActionType, Agent, AggregateView, ArtifactType, Asset, CapabilityGrant, CustomTool, Datasource,
    Environment, EventType, JobType, LinkType, ObjectType, PolicyRule, Role, Spec, Trait,
    UploadFlow, Workspace,
};

/// A fluent builder for a complete Tesela ontology definition.
///
/// Chain method calls to add ontology elements, then call [`App::compile`]
/// to validate and normalize the spec.
pub struct App {
    spec: Spec,
}

impl App {
    /// Create a new app with the given workspace API name.
    pub fn new(workspace: impl AsRef<str>) -> Self {
        Self {
            spec: Spec {
                workspace: Workspace {
                    api_name: ApiName::new_unchecked(workspace.as_ref()),
                    display: None,
                    metadata: None,
                },
                ..Default::default()
            },
        }
    }

    /// Set a human-readable display name for the workspace.
    pub fn display(mut self, name: impl Into<String>) -> Self {
        self.spec.workspace.display = Some(name.into());
        self
    }

    /// Add a datasource definition.
    pub fn datasource(mut self, ds: Datasource) -> Self {
        self.spec.datasources.push(ds);
        self
    }

    /// Add an object type definition.
    pub fn object_type(mut self, ot: ObjectType) -> Self {
        self.spec.object_types.push(ot);
        self
    }

    /// Add a reusable trait definition.
    pub fn trait_def(mut self, t: Trait) -> Self {
        self.spec.traits.push(t);
        self
    }

    /// Add a link type definition.
    pub fn link(mut self, lt: LinkType) -> Self {
        self.spec.link_types.push(lt);
        self
    }

    /// Add an action type definition.
    pub fn action(mut self, a: ActionType) -> Self {
        self.spec.actions.push(a);
        self
    }

    /// Add a role definition.
    pub fn role(mut self, r: Role) -> Self {
        self.spec.roles.push(r);
        self
    }

    /// Add a policy rule.
    pub fn policy(mut self, p: PolicyRule) -> Self {
        self.spec.policies.push(p);
        self
    }

    /// Add an agent definition.
    pub fn agent(mut self, a: Agent) -> Self {
        self.spec.agents.push(a);
        self
    }

    /// Add a custom tool definition.
    pub fn custom_tool(mut self, ct: CustomTool) -> Self {
        self.spec.custom_tools.push(ct);
        self
    }

    /// Add an environment configuration.
    pub fn environment(mut self, e: Environment) -> Self {
        self.spec.environments.push(e);
        self
    }

    /// Add an asset definition.
    pub fn asset(mut self, a: Asset) -> Self {
        self.spec.assets.push(a);
        self
    }

    /// Add an artifact type definition.
    pub fn artifact_type(mut self, a: ArtifactType) -> Self {
        self.spec.artifact_types.push(a);
        self
    }

    /// Add an upload flow definition.
    pub fn upload_flow(mut self, u: UploadFlow) -> Self {
        self.spec.upload_flows.push(u);
        self
    }

    /// Add a job type definition.
    pub fn job_type(mut self, j: JobType) -> Self {
        self.spec.job_types.push(j);
        self
    }

    /// Add an event type definition.
    pub fn event_type(mut self, e: EventType) -> Self {
        self.spec.event_types.push(e);
        self
    }

    /// Add a capability grant definition.
    pub fn capability_grant(mut self, c: CapabilityGrant) -> Self {
        self.spec.capability_grants.push(c);
        self
    }

    /// Add an aggregate view definition.
    pub fn aggregate_view(mut self, a: AggregateView) -> Self {
        self.spec.aggregate_views.push(a);
        self
    }

    /// Return the raw spec without compiling.
    pub fn spec(&self) -> &Spec {
        &self.spec
    }

    /// Compile the spec using the default pipeline.
    pub fn compile(&self) -> CompileResult {
        let compiler = Compiler::default_pipeline();
        compiler.compile(&self.prepare_spec())
    }

    fn prepare_spec(&self) -> Spec {
        self.spec.clone()
    }

    /// Compile and serialize the result to a pretty-printed JSON string.
    pub fn compile_json(&self) -> Result<String, Error> {
        let result = self.compile();
        let spec = result.spec.ok_or_else(|| {
            let diag_msgs: Vec<String> = result
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect();
            Error::validation(diag_msgs.join("; "))
        })?;
        spec.to_json_pretty()
    }

    /// Consume the app, compile the spec, and create a live runtime.
    #[cfg(feature = "native-runtime")]
    pub fn into_runtime(
        self,
        opts: tesela_runtime::runtime::RuntimeOptions,
    ) -> Result<std::sync::Arc<tesela_runtime::runtime::Runtime>, Error> {
        let result = self.compile();
        let spec = result.spec.ok_or_else(|| {
            let msgs: Vec<String> = result
                .diagnostics
                .iter()
                .map(|d| d.message.clone())
                .collect();
            Error::validation(msgs.join("; "))
        })?;
        tesela_runtime::runtime::Runtime::new(spec, opts)
    }
}

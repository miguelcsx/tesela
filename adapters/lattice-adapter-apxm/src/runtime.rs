use lattice_core::{Error, Value};
use lattice_runtime::{
    ports::AgentRuntime,
    query::Actor,
};

/// APXM agent runtime that delegates to an APXM HTTP service.
pub struct ApxmAgentRuntime {
    base_url: String,
    client: reqwest::Client,
}

impl ApxmAgentRuntime {
    /// Create a new APXM adapter pointing at the given base URL.
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Create with a custom HTTP client.
    pub fn with_client(base_url: impl Into<String>, client: reqwest::Client) -> Self {
        Self {
            base_url: base_url.into(),
            client,
        }
    }
}

impl AgentRuntime for ApxmAgentRuntime {
    fn start_run(
        &self,
        _agent: &lattice_ir::Agent,
        _input: Value,
        _actor: &Actor,
    ) -> Result<String, Error> {
        Err(Error::unsupported(format!(
            "apxm start_run not yet implemented — requires APXM service at {}",
            self.base_url
        )))
    }

    fn get_run(&self, run_id: &str) -> Result<lattice_ir::AgentRun, Error> {
        let _url = self.client.get(format!("{}/runs/{}", self.base_url, run_id));
        Err(Error::unsupported(format!(
            "apxm get_run not yet implemented — requires APXM service at {}",
            self.base_url
        )))
    }
}

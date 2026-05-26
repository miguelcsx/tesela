//! Branch / draft spec management.

use crate::ports::BranchStore;
use tesela_core::Error;
use tesela_ir::{Branch, BranchStatus, Spec};
use std::collections::HashMap;
use std::sync::RwLock;

/// In-memory [`BranchStore`] for development and testing.
pub struct MemoryBranchStore {
    branches: RwLock<HashMap<String, Branch>>,
}

impl MemoryBranchStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            branches: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for MemoryBranchStore {
    fn default() -> Self {
        Self::new()
    }
}

impl BranchStore for MemoryBranchStore {
    fn create_branch(&self, base: &Spec, display: &str, author: &str) -> Result<Branch, Error> {
        let id = uuid::Uuid::new_v4().to_string();
        let base_spec_hash = base.hash();
        let branch = Branch {
            id: id.clone(),
            display: display.to_string(),
            base_spec_hash,
            draft_spec: base.clone(),
            status: BranchStatus::Draft,
            created_at: chrono::Utc::now().to_rfc3339(),
            author: author.to_string(),
        };
        self.branches
            .write()
            .map_err(|_| Error::internal("branch store lock poisoned"))?
            .insert(id, branch.clone());
        Ok(branch)
    }

    fn get_branch(&self, id: &str) -> Result<Option<Branch>, Error> {
        Ok(self
            .branches
            .read()
            .map_err(|_| Error::internal("branch store lock poisoned"))?
            .get(id)
            .cloned())
    }

    fn update_draft(&self, id: &str, spec: Spec) -> Result<(), Error> {
        let mut map = self
            .branches
            .write()
            .map_err(|_| Error::internal("branch store lock poisoned"))?;
        let branch = map
            .get_mut(id)
            .ok_or_else(|| Error::not_found("branch", id))?;
        if branch.status != BranchStatus::Draft && branch.status != BranchStatus::Review {
            return Err(Error::validation(format!(
                "branch '{}' is not open for edits (status: {:?})",
                id, branch.status
            )));
        }
        branch.draft_spec = spec;
        Ok(())
    }

    fn set_status(&self, id: &str, status: BranchStatus) -> Result<(), Error> {
        let mut map = self
            .branches
            .write()
            .map_err(|_| Error::internal("branch store lock poisoned"))?;
        let branch = map
            .get_mut(id)
            .ok_or_else(|| Error::not_found("branch", id))?;
        branch.status = status;
        Ok(())
    }

    fn list_branches(&self) -> Result<Vec<Branch>, Error> {
        Ok(self
            .branches
            .read()
            .map_err(|_| Error::internal("branch store lock poisoned"))?
            .values()
            .cloned()
            .collect())
    }

    fn delete_branch(&self, id: &str) -> Result<(), Error> {
        self.branches
            .write()
            .map_err(|_| Error::internal("branch store lock poisoned"))?
            .remove(id)
            .ok_or_else(|| Error::not_found("branch", id))
            .map(|_| ())
    }
}

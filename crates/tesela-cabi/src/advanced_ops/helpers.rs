use crate::handle::*;
use std::sync::Arc;

pub(super) fn runtime_from_handle(handle: u64) -> Result<Arc<tesela_runtime::Runtime>, String> {
    let ht = handles()
        .lock()
        .map_err(|_| "handle table lock poisoned".to_string())?;
    ht.get(handle)
        .cloned()
        .ok_or_else(|| "invalid handle".to_string())
}

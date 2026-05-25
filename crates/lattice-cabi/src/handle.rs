//! Handle table, buffer types, helpers, and memory management exports.

use lattice_compiler::Compiler;
use lattice_core::ApiName;
use lattice_ir::Spec;
use lattice_memory::MemoryBackend;
use lattice_runtime::{
    audit::VecAuditSink,
    ports::{Backend, BackendFactory, BackendRegistry},
    runtime::{Runtime, RuntimeOptions},
};
use serde::{de::DeserializeOwned, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::sync::{Arc, Mutex, OnceLock};
pub(crate) enum Subscription {
    Event(std::sync::mpsc::Receiver<lattice_runtime::query::Event>),
    Change(std::sync::mpsc::Receiver<lattice_runtime::ports::ChangeEvent>),
}

/// A registered C callback with optional user data.
///
/// # Safety
/// The user_data pointer is opaque; callers must ensure it remains valid
/// for the lifetime of the callback registration.
#[derive(Clone, Copy)]
pub(crate) struct CCallback {
    pub callback: CallbackFn,
    pub user_data: *mut c_void,
}

unsafe impl Send for CCallback {}
unsafe impl Sync for CCallback {}

pub(crate) struct HandleTable {
    next: u64,
    runtimes: HashMap<u64, Arc<Runtime>>,
    pub(crate) last_error: String,
    pub(crate) backend_callbacks: HashMap<String, CCallback>,
    pub(crate) action_callbacks: HashMap<String, CCallback>,
    pub(crate) custom_tool_callbacks: HashMap<String, CCallback>,
    pub(crate) action_by_name_callbacks: HashMap<String, CCallback>,
    pub(crate) object_store_callbacks: HashMap<String, CCallback>,
    pub(crate) message_bus_callbacks: HashMap<String, CCallback>,
    pub(crate) run_store_callbacks: HashMap<String, CCallback>,
    pub(crate) capability_callbacks: HashMap<String, CCallback>,
    sub_next: u64,
    pub(crate) subscriptions: HashMap<u64, Subscription>,
}

impl HandleTable {
    fn new() -> Self {
        Self {
            next: 1,
            runtimes: HashMap::new(),
            last_error: String::new(),
            backend_callbacks: HashMap::new(),
            action_callbacks: HashMap::new(),
            custom_tool_callbacks: HashMap::new(),
            action_by_name_callbacks: HashMap::new(),
            object_store_callbacks: HashMap::new(),
            message_bus_callbacks: HashMap::new(),
            run_store_callbacks: HashMap::new(),
            capability_callbacks: HashMap::new(),
            sub_next: 1,
            subscriptions: HashMap::new(),
        }
    }

    pub(crate) fn insert(&mut self, rt: Arc<Runtime>) -> u64 {
        let id = self.next;
        self.next += 1;
        self.runtimes.insert(id, rt);
        self.last_error.clear();
        id
    }

    pub(crate) fn get(&self, handle: u64) -> Option<&Arc<Runtime>> {
        self.runtimes.get(&handle)
    }

    pub(crate) fn remove(&mut self, handle: u64) -> Option<Arc<Runtime>> {
        self.runtimes.remove(&handle)
    }

    pub(crate) fn set_error(&mut self, msg: &str) {
        self.last_error = msg.to_string();
    }

    pub(crate) fn insert_sub(&mut self, sub: Subscription) -> u64 {
        let id = self.sub_next;
        self.sub_next += 1;
        self.subscriptions.insert(id, sub);
        id
    }

    pub(crate) fn get_sub(&mut self, handle: u64) -> Option<&mut Subscription> {
        self.subscriptions.get_mut(&handle)
    }

    pub(crate) fn remove_sub(&mut self, handle: u64) -> Option<Subscription> {
        self.subscriptions.remove(&handle)
    }
}

pub(crate) fn handles() -> &'static Mutex<HandleTable> {
    static HANDLES: OnceLock<Mutex<HandleTable>> = OnceLock::new();
    HANDLES.get_or_init(|| Mutex::new(HandleTable::new()))
}

macro_rules! lock_handles {
    (or return $default:expr) => {
        match $crate::handle::handles().lock() {
            Ok(guard) => guard,
            Err(_) => return $default,
        }
    };
    (or return) => {
        match $crate::handle::handles().lock() {
            Ok(guard) => guard,
            Err(_) => return,
        }
    };
}
pub(crate) use lock_handles;

pub(crate) fn c_error_string(msg: &str) -> *mut c_char {
    CString::new(msg)
        .map(CString::into_raw)
        .unwrap_or(std::ptr::null_mut())
}

/// A heap-allocated byte buffer returned to the caller.
///
/// Free with [`lattice_buffer_free`].
#[repr(C)]
pub struct LatticeBuffer {
    /// Pointer to data (may be null when `len == 0`).
    pub data: *mut c_char,
    /// Byte length of the data.
    pub len: c_int,
}

impl LatticeBuffer {
    pub(crate) fn empty() -> Self {
        Self {
            data: std::ptr::null_mut(),
            len: 0,
        }
    }

    pub(crate) fn from_bytes(bytes: Vec<u8>) -> Self {
        if bytes.is_empty() {
            return Self::empty();
        }
        let len = bytes.len() as c_int;
        let mut boxed = bytes.into_boxed_slice();
        let data = boxed.as_mut_ptr() as *mut c_char;
        std::mem::forget(boxed);
        Self { data, len }
    }
}

/// Callback function type for user-registered backends and action handlers.
pub type CallbackFn = extern "C" fn(
    user_data: *mut c_void,
    req_json: *const c_char,
    req_len: c_int,
    out_len: *mut c_int,
) -> *mut c_char;

/// # Safety
/// `ptr` must be valid for `len` bytes.
pub(crate) unsafe fn read_json_str(ptr: *const c_char, len: c_int) -> Option<String> {
    if ptr.is_null() || len <= 0 {
        return None;
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    std::str::from_utf8(slice).ok().map(|s| s.to_string())
}

/// # Safety
/// `ptr` must be valid for `len` bytes.
pub(crate) unsafe fn decode_json<T: DeserializeOwned>(
    ptr: *const c_char,
    len: c_int,
) -> Result<T, String> {
    let s = unsafe { read_json_str(ptr, len) }
        .ok_or_else(|| "null or empty JSON input".to_string())?;
    serde_json::from_str(&s).map_err(|e| e.to_string())
}

pub(crate) fn marshal_result<T: Serialize>(
    value: &T,
    handle_table: &mut HandleTable,
) -> LatticeBuffer {
    match serde_json::to_vec(value) {
        Ok(bytes) => LatticeBuffer::from_bytes(bytes),
        Err(e) => {
            handle_table.set_error(&e.to_string());
            LatticeBuffer::empty()
        }
    }
}

pub(crate) fn set_last_error(msg: &str) {
    if let Ok(mut ht) = handles().lock() {
        ht.set_error(msg);
    }
}

pub(crate) fn marshal_result_global<T: Serialize>(value: &T) -> LatticeBuffer {
    match serde_json::to_vec(value) {
        Ok(bytes) => LatticeBuffer::from_bytes(bytes),
        Err(e) => {
            set_last_error(&e.to_string());
            LatticeBuffer::empty()
        }
    }
}

pub(crate) unsafe fn parse_spec(ptr: *const c_char, len: c_int) -> Result<Spec, String> {
    let s = unsafe { read_json_str(ptr, len) }.ok_or_else(|| "null spec JSON".to_string())?;
    let spec: Spec = serde_json::from_str(&s).map_err(|e| format!("spec parse error: {}", e))?;
    let result = Compiler::default_pipeline().compile(&spec);
    if !result.is_valid {
        let msgs: Vec<_> = result
            .diagnostics
            .iter()
            .map(|d| d.message.clone())
            .collect();
        return Err(msgs.join("; "));
    }
    result
        .spec
        .ok_or_else(|| "compiler produced no spec".to_string())
}

pub(crate) fn build_runtime(
    spec: Spec,
    _backend_callbacks: &HashMap<String, CCallback>,
) -> Result<Arc<Runtime>, String> {
    let registry = Arc::new(CAbiBackendRegistry::new(&spec));

    let opts = RuntimeOptions {
        backend_registry: Some(registry),
        audit_sink: Some(Arc::new(VecAuditSink::new())),
        object_store: Some(Arc::new(CAbiObjectStore)),
        message_bus: Some(Arc::new(CAbiMessageBus)),
        run_store: Some(Arc::new(CAbiRunStore)),
        capability_issuer: Some(Arc::new(CAbiCapabilityIssuer)),
        allow_dev_defaults: true,
        ..Default::default()
    };

    Runtime::new(spec, opts).map_err(|e| e.to_string())
}

fn call_named_callback<T: serde::de::DeserializeOwned>(
    callbacks: &HashMap<String, CCallback>,
    key: &str,
    req: serde_json::Value,
) -> Result<T, lattice_core::Error> {
    let callback = callbacks.get(key).copied().ok_or_else(|| {
        lattice_core::Error::adapter(format!("no C ABI callback registered for '{}'", key))
    })?;
    let req_bytes = serde_json::to_vec(&req)
        .map_err(|e| lattice_core::Error::internal(format!("callback request encode: {}", e)))?;
    let mut out_len: c_int = 0;
    let ptr = (callback.callback)(
        callback.user_data,
        req_bytes.as_ptr() as *const c_char,
        req_bytes.len() as c_int,
        &mut out_len,
    );
    if ptr.is_null() || out_len <= 0 {
        return Err(lattice_core::Error::adapter("callback returned no data"));
    }
    let slice = unsafe { std::slice::from_raw_parts(ptr as *const u8, out_len as usize) };
    let value = serde_json::from_slice(slice)
        .map_err(|e| lattice_core::Error::adapter(format!("callback response decode: {}", e)))?;
    unsafe {
        drop(Vec::from_raw_parts(
            ptr as *mut u8,
            out_len as usize,
            out_len as usize,
        ));
    }
    Ok(value)
}

fn callback_key_from_metadata(metadata: &BTreeMap<String, lattice_core::Value>) -> String {
    metadata
        .get("_store")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string()
}

struct CAbiObjectStore;

impl lattice_runtime::ports::ObjectStore for CAbiObjectStore {
    fn signed_upload_url(
        &self,
        path: &str,
        ttl_secs: u64,
        metadata: &BTreeMap<String, lattice_core::Value>,
    ) -> Result<lattice_runtime::query::SignedUpload, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        let key = callback_key_from_metadata(metadata);
        call_named_callback(
            &ht.object_store_callbacks,
            &key,
            serde_json::json!({"operation":"signed_upload_url","path":path,"ttl_secs":ttl_secs,"metadata":metadata}),
        )
    }

    fn signed_read_url(
        &self,
        path: &str,
        ttl_secs: u64,
        metadata: &BTreeMap<String, lattice_core::Value>,
    ) -> Result<lattice_runtime::query::ArtifactLocator, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        let key = callback_key_from_metadata(metadata);
        call_named_callback(
            &ht.object_store_callbacks,
            &key,
            serde_json::json!({"operation":"signed_read_url","path":path,"ttl_secs":ttl_secs,"metadata":metadata}),
        )
    }

    fn stat(
        &self,
        path: &str,
    ) -> Result<lattice_runtime::query::ObjectMetadata, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.object_store_callbacks,
            "default",
            serde_json::json!({"operation":"stat","path":path}),
        )
    }

    fn list(
        &self,
        prefix: &str,
    ) -> Result<Vec<lattice_runtime::query::ObjectMetadata>, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.object_store_callbacks,
            "default",
            serde_json::json!({"operation":"list","prefix":prefix}),
        )
    }

    fn delete(&self, path: &str) -> Result<(), lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        let _: serde_json::Value = call_named_callback(
            &ht.object_store_callbacks,
            "default",
            serde_json::json!({"operation":"delete","path":path}),
        )?;
        Ok(())
    }
}

struct CAbiMessageBus;

impl lattice_runtime::ports::MessageBus for CAbiMessageBus {
    fn publish_message(
        &self,
        event_type: &ApiName,
        event: lattice_runtime::query::Event,
    ) -> Result<String, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.message_bus_callbacks,
            "default",
            serde_json::json!({"operation":"publish_message","event_type":event_type,"event":event}),
        )
    }

    fn dequeue_message(
        &self,
        event_type: &ApiName,
    ) -> Result<Option<lattice_runtime::query::Event>, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.message_bus_callbacks,
            "default",
            serde_json::json!({"operation":"dequeue_message","event_type":event_type}),
        )
    }

    fn ack_message(
        &self,
        event_type: &ApiName,
        message_id: &str,
    ) -> Result<(), lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        let _: serde_json::Value = call_named_callback(
            &ht.message_bus_callbacks,
            "default",
            serde_json::json!({"operation":"ack_message","event_type":event_type,"message_id":message_id}),
        )?;
        Ok(())
    }

    fn nack_message(
        &self,
        event_type: &ApiName,
        message_id: &str,
        requeue: bool,
    ) -> Result<(), lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        let _: serde_json::Value = call_named_callback(
            &ht.message_bus_callbacks,
            "default",
            serde_json::json!({"operation":"nack_message","event_type":event_type,"message_id":message_id,"requeue":requeue}),
        )?;
        Ok(())
    }
}

struct CAbiRunStore;

impl lattice_runtime::ports::RunStore for CAbiRunStore {
    fn create_or_reuse(
        &self,
        run: lattice_runtime::query::RunRecord,
    ) -> Result<lattice_runtime::query::RunRecord, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.run_store_callbacks,
            "default",
            serde_json::json!({"operation":"create_or_reuse","run":run}),
        )
    }

    fn get_run(
        &self,
        run_id: &str,
    ) -> Result<Option<lattice_runtime::query::RunRecord>, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.run_store_callbacks,
            "default",
            serde_json::json!({"operation":"get_run","run_id":run_id}),
        )
    }

    fn update_run(
        &self,
        run: lattice_runtime::query::RunRecord,
    ) -> Result<lattice_runtime::query::RunRecord, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.run_store_callbacks,
            "default",
            serde_json::json!({"operation":"update_run","run":run}),
        )
    }
}

struct CAbiCapabilityIssuer;

impl lattice_runtime::ports::CapabilityIssuer for CAbiCapabilityIssuer {
    fn issue_capability(
        &self,
        grant: &lattice_ir::CapabilityGrant,
        actor: &lattice_runtime::query::Actor,
        constraints: BTreeMap<String, lattice_core::Value>,
    ) -> Result<lattice_runtime::query::CapabilityToken, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.capability_callbacks,
            "default",
            serde_json::json!({"operation":"issue_capability","grant":grant,"actor":actor,"constraints":constraints}),
        )
    }

    fn verify_capability(
        &self,
        token: &str,
    ) -> Result<lattice_runtime::query::CapabilityToken, lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        call_named_callback(
            &ht.capability_callbacks,
            "default",
            serde_json::json!({"operation":"verify_capability","token":token}),
        )
    }

    fn revoke_capability(&self, token_id: &str) -> Result<(), lattice_core::Error> {
        let ht = handles()
            .lock()
            .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
        let _: serde_json::Value = call_named_callback(
            &ht.capability_callbacks,
            "default",
            serde_json::json!({"operation":"revoke_capability","token_id":token_id}),
        )?;
        Ok(())
    }
}

struct CAbiBackendRegistry {
    datasources: HashMap<ApiName, String>,
    memory_backends: HashMap<ApiName, Arc<dyn Backend>>,
}

impl CAbiBackendRegistry {
    fn new(spec: &Spec) -> Self {
        Self {
            datasources: spec
                .datasources
                .iter()
                .map(|ds| (ds.api_name.clone(), ds.adapter_type.clone()))
                .collect(),
            memory_backends: spec
                .datasources
                .iter()
                .filter(|ds| ds.adapter_type == "memory")
                .map(|ds| {
                    (
                        ds.api_name.clone(),
                        MemoryBackend::new() as Arc<dyn Backend>,
                    )
                })
                .collect(),
        }
    }
}

impl BackendRegistry for CAbiBackendRegistry {
    fn acquire(&self, ds_name: &ApiName) -> Result<Box<dyn Backend>, lattice_core::Error> {
        let adapter_type = self
            .datasources
            .get(ds_name)
            .ok_or_else(|| lattice_core::Error::not_found("datasource", ds_name))?;

        if adapter_type == "memory" {
            let backend = self
                .memory_backends
                .get(ds_name)
                .cloned()
                .ok_or_else(|| lattice_core::Error::not_found("datasource", ds_name))?;
            return Ok(Box::new(CAbiArcBackendRef(backend)));
        }

        let callback = {
            let ht = handles()
                .lock()
                .map_err(|_| lattice_core::Error::internal("handle table lock poisoned"))?;
            ht.backend_callbacks.get(adapter_type).copied()
        }
        .ok_or_else(|| {
            lattice_core::Error::adapter(format!(
                "no C ABI backend callback registered for adapter type '{}' (datasource '{}')",
                adapter_type, ds_name
            ))
        })?;

        Ok(Box::new(crate::callback_backend::CAbiBackend::new(
            adapter_type.clone(),
            callback,
        )))
    }

    fn register_factory(
        &self,
        _ds_name: ApiName,
        _factory: Box<dyn BackendFactory>,
    ) -> Result<(), lattice_core::Error> {
        Err(lattice_core::Error::unsupported(
            "C ABI backend registry uses runtime callback registration",
        ))
    }
}

struct CAbiArcBackendRef(Arc<dyn Backend>);

impl Backend for CAbiArcBackendRef {
    fn backend_type(&self) -> &str {
        self.0.backend_type()
    }
    fn capabilities(&self) -> lattice_runtime::query::BackendCapabilities {
        self.0.capabilities()
    }
    fn close(&self) -> Result<(), lattice_core::Error> {
        self.0.close()
    }
    fn as_searcher(&self) -> Option<&dyn lattice_runtime::ports::Searcher> {
        self.0.as_searcher()
    }
    fn as_getter(&self) -> Option<&dyn lattice_runtime::ports::Getter> {
        self.0.as_getter()
    }
    fn as_mutator(&self) -> Option<&dyn lattice_runtime::ports::Mutator> {
        self.0.as_mutator()
    }
    fn as_aggregator(&self) -> Option<&dyn lattice_runtime::ports::Aggregator> {
        self.0.as_aggregator()
    }
    fn as_traverser(&self) -> Option<&dyn lattice_runtime::ports::Traverser> {
        self.0.as_traverser()
    }
    fn as_bulk_loader(&self) -> Option<&dyn lattice_runtime::ports::BulkLoader> {
        self.0.as_bulk_loader()
    }
    fn as_rollbacker(&self) -> Option<&dyn lattice_runtime::ports::Rollbacker> {
        self.0.as_rollbacker()
    }
    fn as_explainer(&self) -> Option<&dyn lattice_runtime::ports::SearchExplainer> {
        self.0.as_explainer()
    }
}

pub(crate) unsafe fn extract_actor(
    ptr: *const c_char,
    len: c_int,
) -> lattice_runtime::query::Actor {
    if ptr.is_null() || len <= 0 {
        return default_actor();
    }
    match unsafe { decode_json::<lattice_runtime::query::Actor>(ptr, len) } {
        Ok(a) => a,
        Err(_) => default_actor(),
    }
}

pub(crate) fn default_actor() -> lattice_runtime::query::Actor {
    lattice_runtime::query::Actor {
        user_id: "native-sdk".to_string(),
        roles: Vec::new(),
        claims: Default::default(),
    }
}

pub(crate) unsafe fn parse_api_name(ptr: *const c_char) -> Result<ApiName, String> {
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .map_err(|e| e.to_string())
        .and_then(|s| s.parse::<ApiName>().map_err(|e| e.to_string()))
}

/// Return the last error message as a heap-allocated C string.
#[unsafe(no_mangle)]
pub extern "C" fn lattice_last_error() -> *mut c_char {
    let ht = lock_handles!(or return std::ptr::null_mut());
    if ht.last_error.is_empty() {
        return std::ptr::null_mut();
    }
    match CString::new(ht.last_error.clone()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Free a C string previously returned by this library.
///
/// # Safety
/// `ptr` must have been returned by this library and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_string_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        drop(unsafe { CString::from_raw(ptr) });
    }
}

/// Free a [`LatticeBuffer`] previously returned by this library.
///
/// # Safety
/// `buf` must have been returned by this library and not already freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn lattice_buffer_free(buf: LatticeBuffer) {
    if !buf.data.is_null() && buf.len > 0 {
        unsafe {
            let slice = std::slice::from_raw_parts_mut(buf.data as *mut u8, buf.len as usize);
            drop(Box::from_raw(slice as *mut [u8]));
        }
    }
}

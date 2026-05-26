from __future__ import annotations

import ctypes
import json
import os
from typing import Any, Callable

from tesela.runtime._types import (
    NativeError,
    Record,
    Page,
    MutationResult,
    ActionResult,
    ExplainPlan,
    HealthStatus,
    Subscription,
    _wrap_record,
    _wrap_page,
    _wrap_mutation,
    _wrap_action,
    _wrap_explain,
    _wrap_health,
)
from tesela.runtime._ffi import (
    _Buffer,
    _Callback,
    _load_library,
    _json_bytes,
    _ptr,
    _libc,
)


class NativeRuntime:

    def __init__(
        self,
        spec: dict[str, Any] | str | bytes,
        *,
        library_path: str | os.PathLike[str] | None = None,
    ):
        self._lib = _load_library(library_path)
        self._callbacks: list[_Callback] = []
        raw = _json_bytes(spec)
        handle = self._lib.tesela_runtime_new_from_spec_json(_ptr(raw), len(raw))
        if not handle:
            raise NativeError(self._last_error())
        self._handle = int(handle)

    @classmethod
    def from_app(
        cls, app: Any, *, library_path: str | os.PathLike[str] | None = None
    ) -> NativeRuntime:
        return cls(app.compile_json(), library_path=library_path)

    def close(self) -> None:
        if self._handle:
            self._lib.tesela_runtime_release(self._handle)
            self._handle = 0

    def shutdown(self) -> None:
        err = self._lib.tesela_runtime_shutdown(self._handle)
        self._raise_if_error(err)

    def spec(self) -> dict[str, Any]:
        return self._call_buffer(self._lib.tesela_runtime_spec_json, self._handle)

    def apply_spec(self, spec: dict[str, Any] | str | bytes) -> dict[str, Any]:
        raw = _json_bytes(spec)
        return self._call_buffer(
            self._lib.tesela_runtime_apply_spec_json, self._handle, _ptr(raw), len(raw)
        )

    def register_backend(
        self, adapter_type: str, handler: Callable[[dict[str, Any]], dict[str, Any]]
    ) -> None:
        cb = self._make_callback(handler)
        err = self._lib.tesela_runtime_register_backend(
            self._handle, adapter_type.encode(), cb, None
        )
        self._raise_if_error(err)
        self._callbacks.append(cb)

    def register_action_handler(
        self, kind: str, handler: Callable[[dict[str, Any]], dict[str, Any]]
    ) -> None:
        cb = self._make_callback(handler)
        err = self._lib.tesela_runtime_register_action_handler(
            self._handle, kind.encode(), cb, None
        )
        self._raise_if_error(err)
        self._callbacks.append(cb)

    def register_custom_tool(
        self, name: str, handler: Callable[[dict[str, Any]], dict[str, Any]]
    ) -> None:
        cb = self._make_callback(handler)
        err = self._lib.tesela_runtime_register_custom_tool(
            self._handle, name.encode(), cb, None
        )
        self._raise_if_error(err)
        self._callbacks.append(cb)

    def register_object_store(
        self, handler: Callable[[dict[str, Any]], dict[str, Any]], *, name: str = "default"
    ) -> None:
        cb = self._make_callback(handler)
        err = self._lib.tesela_runtime_register_object_store(self._handle, name.encode(), cb, None)
        self._raise_if_error(err)
        self._callbacks.append(cb)

    def register_message_bus(
        self, handler: Callable[[dict[str, Any]], dict[str, Any]], *, name: str = "default"
    ) -> None:
        cb = self._make_callback(handler)
        err = self._lib.tesela_runtime_register_message_bus(self._handle, name.encode(), cb, None)
        self._raise_if_error(err)
        self._callbacks.append(cb)

    def register_run_store(
        self, handler: Callable[[dict[str, Any]], dict[str, Any]], *, name: str = "default"
    ) -> None:
        cb = self._make_callback(handler)
        err = self._lib.tesela_runtime_register_run_store(self._handle, name.encode(), cb, None)
        self._raise_if_error(err)
        self._callbacks.append(cb)

    def register_capability_issuer(
        self, handler: Callable[[dict[str, Any]], dict[str, Any]], *, name: str = "default"
    ) -> None:
        cb = self._make_callback(handler)
        err = self._lib.tesela_runtime_register_capability_issuer(self._handle, name.encode(), cb, None)
        self._raise_if_error(err)
        self._callbacks.append(cb)

    def search(
        self,
        object_type: str,
        query: dict[str, Any] | None = None,
        *,
        actor: dict[str, Any] | None = None,
    ) -> Page:
        actor_raw = _json_bytes(actor) if actor else b""
        query_raw = _json_bytes(query or {})
        return _wrap_page(self._call_buffer(
            self._lib.tesela_runtime_search_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(query_raw),
            len(query_raw),
        ))

    def get(
        self, object_type: str, primary_key: Any, *, actor: dict[str, Any] | None = None
    ) -> Record | None:
        actor_raw = _json_bytes(actor) if actor else b""
        pk_raw = _json_bytes(primary_key)
        data = self._call_buffer(
            self._lib.tesela_runtime_get_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(pk_raw),
            len(pk_raw),
        )
        return _wrap_record(data)

    def mutate(
        self,
        object_type: str,
        mutation: dict[str, Any],
        *,
        actor: dict[str, Any] | None = None,
    ) -> MutationResult:
        actor_raw = _json_bytes(actor) if actor else b""
        mutation_raw = _json_bytes(mutation)
        return _wrap_mutation(self._call_buffer(
            self._lib.tesela_runtime_mutate_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(mutation_raw),
            len(mutation_raw),
        ))

    def execute_action(
        self,
        action: str,
        input: dict[str, Any] | None = None,
        *,
        actor: dict[str, Any] | None = None,
    ) -> ActionResult:
        actor_raw = _json_bytes(actor) if actor else b""
        input_raw = _json_bytes(input or {})
        return _wrap_action(self._call_buffer(
            self._lib.tesela_runtime_execute_action_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            action.encode(),
            _ptr(input_raw),
            len(input_raw),
        ))

    def explain(
        self,
        object_type: str,
        query: dict[str, Any] | None = None,
        *,
        actor: dict[str, Any] | None = None,
    ) -> ExplainPlan:
        actor_raw = _json_bytes(actor) if actor else b""
        query_raw = _json_bytes(query or {})
        return _wrap_explain(self._call_buffer(
            self._lib.tesela_runtime_explain_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(query_raw),
            len(query_raw),
        ))

    def traverse(
        self,
        link_type: str,
        query: dict[str, Any] | None = None,
        *,
        actor: dict[str, Any] | None = None,
    ) -> Page:
        actor_raw = _json_bytes(actor) if actor else b""
        query_raw = _json_bytes(query or {})
        return _wrap_page(self._call_buffer(
            self._lib.tesela_runtime_traverse_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            link_type.encode(),
            _ptr(query_raw),
            len(query_raw),
        ))

    def aggregate(
        self,
        object_type: str,
        query: dict[str, Any] | None = None,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        query_raw = _json_bytes(query or {})
        return self._call_buffer(
            self._lib.tesela_runtime_aggregate_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(query_raw),
            len(query_raw),
        )

    def aggregate_view(
        self,
        view_name: str,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        return self._call_buffer(
            self._lib.tesela_runtime_aggregate_view_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            view_name.encode(),
        )

    def issue_capability(
        self,
        grant: str,
        constraints: dict[str, Any] | None = None,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        body_raw = _json_bytes(constraints or {})
        return self._call_buffer(
            self._lib.tesela_runtime_issue_capability_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            grant.encode(),
            _ptr(body_raw),
            len(body_raw),
        )

    def authorize_artifact_read(
        self,
        artifact: str,
        params: dict[str, Any],
        *,
        ttl_seconds: int = 300,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        params_raw = _json_bytes(params)
        return self._call_buffer(
            self._lib.tesela_runtime_authorize_artifact_read_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            artifact.encode(),
            _ptr(params_raw),
            len(params_raw),
            ttl_seconds,
        )

    def initiate_upload_flow(
        self,
        flow: str,
        params: dict[str, Any],
        *,
        ttl_seconds: int = 300,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        params_raw = _json_bytes(params)
        return self._call_buffer(
            self._lib.tesela_runtime_initiate_upload_flow_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            flow.encode(),
            _ptr(params_raw),
            len(params_raw),
            ttl_seconds,
        )

    def start_job(
        self,
        job: str,
        input: dict[str, Any] | None = None,
        *,
        idempotency_key: str = "",
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        body_raw = _json_bytes({"input": input or {}, "idempotency_key": idempotency_key})
        return self._call_buffer(
            self._lib.tesela_runtime_start_job_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            job.encode(),
            _ptr(body_raw),
            len(body_raw),
        )

    def complete_upload_flow(
        self,
        flow: str,
        path: str,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        body_raw = _json_bytes({"path": path})
        return self._call_buffer(
            self._lib.tesela_runtime_complete_upload_flow_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            flow.encode(),
            _ptr(body_raw),
            len(body_raw),
        )

    def load_upload_flow(
        self,
        flow: str,
        records: list[dict[str, Any]],
        *,
        load_id: str = "",
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        body_raw = _json_bytes({"records": records, "load_id": load_id})
        return self._call_buffer(
            self._lib.tesela_runtime_load_upload_flow_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            flow.encode(),
            _ptr(body_raw),
            len(body_raw),
        )

    def upload(
        self,
        object_type: str,
        content: bytes | str,
        *,
        format: str = "csv",
        mappings: list[dict[str, Any]] | None = None,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        req_raw = _json_bytes({"format": format, "mappings": mappings or []})
        content_raw = content.encode() if isinstance(content, str) else content
        return self._call_buffer(
            self._lib.tesela_runtime_upload_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(req_raw),
            len(req_raw),
            _ptr(content_raw),
            len(content_raw),
        )

    def rollback_upload(
        self, object_type: str, load_id: str, *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        body_raw = _json_bytes({"load_id": load_id})
        return self._call_buffer(
            self._lib.tesela_runtime_rollback_upload_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(body_raw),
            len(body_raw),
        )

    def agent_start(
        self,
        agent: str,
        input: dict[str, Any] | None = None,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        input_raw = _json_bytes(input or {})
        return self._call_buffer(
            self._lib.tesela_runtime_agent_start_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            agent.encode(),
            _ptr(input_raw),
            len(input_raw),
        )

    def agent_get_run(self, run_id: str) -> dict[str, Any]:
        return self._call_buffer(
            self._lib.tesela_runtime_agent_get_run_json, self._handle, run_id.encode()
        )

    def configure_apxm_agent_runtime(
        self,
        base_url: str,
        *,
        timeout_seconds: int = 120,
    ) -> dict[str, Any]:
        config_raw = _json_bytes(
            {"base_url": base_url, "timeout_seconds": timeout_seconds}
        )
        return self._call_buffer(
            self._lib.tesela_runtime_configure_apxm_agent_runtime_json,
            self._handle,
            _ptr(config_raw),
            len(config_raw),
        )

    def health(self) -> HealthStatus:
        return _wrap_health(self._call_buffer(self._lib.tesela_runtime_health_json, self._handle))

    def capabilities(self) -> dict[str, Any]:
        return self._call_buffer(self._lib.tesela_runtime_capabilities_json, self._handle)

    def vector_search(
        self,
        query: dict[str, Any],
        *,
        actor: dict[str, Any] | None = None,
    ) -> Page:
        actor_raw = _json_bytes(actor) if actor else b""
        query_raw = _json_bytes(query)
        return _wrap_page(self._call_buffer(
            self._lib.tesela_runtime_vector_search_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            _ptr(query_raw),
            len(query_raw),
        ))

    def resolve_object_set(
        self,
        name: str,
        *,
        actor: dict[str, Any] | None = None,
    ) -> Page:
        actor_raw = _json_bytes(actor) if actor else b""
        return _wrap_page(self._call_buffer(
            self._lib.tesela_runtime_resolve_object_set_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            name.encode(),
        ))

    def compose_object_sets(
        self,
        names: list[str],
        op: str = "union",
        *,
        actor: dict[str, Any] | None = None,
    ) -> Page:
        actor_raw = _json_bytes(actor) if actor else b""
        body_raw = _json_bytes({"names": names, "op": op})
        return _wrap_page(self._call_buffer(
            self._lib.tesela_runtime_compose_object_sets_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            _ptr(body_raw),
            len(body_raw),
        ))

    def execute_pipeline(
        self,
        pipeline_name: str,
        mode: str = "incremental",
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        body_raw = _json_bytes({"mode": mode})
        return self._call_buffer(
            self._lib.tesela_runtime_execute_pipeline_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            pipeline_name.encode(),
            _ptr(body_raw),
            len(body_raw),
        )

    def get_lineage(
        self,
        object_type: str,
        pk: Any,
        *,
        depth: int | None = None,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        pk_raw = _json_bytes(pk)
        return self._call_buffer(
            self._lib.tesela_runtime_get_lineage_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
            _ptr(pk_raw),
            len(pk_raw),
            depth or 0,
        )

    def cross_search(
        self,
        queries: list[dict[str, Any]],
        *,
        actor: dict[str, Any] | None = None,
    ) -> Page:
        actor_raw = _json_bytes(actor) if actor else b""
        queries_raw = _json_bytes(queries)
        return _wrap_page(self._call_buffer(
            self._lib.tesela_runtime_cross_search_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            _ptr(queries_raw),
            len(queries_raw),
        ))

    def add_entity(self, kind: str, entity: dict[str, Any]) -> dict[str, Any]:
        entity_raw = _json_bytes(entity)
        return self._call_buffer(
            self._lib.tesela_runtime_add_entity_json,
            self._handle,
            kind.encode(),
            _ptr(entity_raw),
            len(entity_raw),
        )

    def add_object_type(self, ot: dict[str, Any]) -> dict[str, Any]:
        return self.add_entity("object_type", ot)

    def add_link_type(self, lt: dict[str, Any]) -> dict[str, Any]:
        return self.add_entity("link_type", lt)

    def add_action(self, action: dict[str, Any]) -> dict[str, Any]:
        return self.add_entity("action", action)

    def add_policy(self, policy: dict[str, Any]) -> dict[str, Any]:
        return self.add_entity("policy", policy)

    def add_agent(self, agent: dict[str, Any]) -> dict[str, Any]:
        return self.add_entity("agent", agent)

    def add_pipeline(self, pipeline: dict[str, Any]) -> dict[str, Any]:
        return self.add_entity("pipeline", pipeline)

    def remove_entity(self, kind: str, api_name: str) -> dict[str, Any]:
        return self._call_buffer(
            self._lib.tesela_runtime_remove_entity_json,
            self._handle,
            kind.encode(),
            api_name.encode(),
        )

    def create_branch(
        self,
        display: str,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        return self._call_buffer(
            self._lib.tesela_runtime_create_branch_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            display.encode(),
        )

    def update_branch_spec(
        self,
        branch_id: str,
        spec: dict[str, Any] | str | bytes,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        spec_raw = _json_bytes(spec)
        return self._call_buffer(
            self._lib.tesela_runtime_update_branch_spec_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            branch_id.encode(),
            _ptr(spec_raw),
            len(spec_raw),
        )

    def merge_branch(
        self,
        branch_id: str,
        *,
        actor: dict[str, Any] | None = None,
    ) -> dict[str, Any]:
        actor_raw = _json_bytes(actor) if actor else b""
        return self._call_buffer(
            self._lib.tesela_runtime_merge_branch_json,
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            branch_id.encode(),
        )

    def list_branches(self) -> dict[str, Any]:
        return self._call_buffer(
            self._lib.tesela_runtime_list_branches_json, self._handle
        )

    def apply_spec_with_migration(
        self, spec: dict[str, Any] | str | bytes
    ) -> dict[str, Any]:
        raw = _json_bytes(spec)
        return self._call_buffer(
            self._lib.tesela_runtime_apply_spec_with_migration_json,
            self._handle,
            _ptr(raw),
            len(raw),
        )

    def schema_graph(self) -> dict[str, Any]:
        return self._call_buffer(
            self._lib.tesela_runtime_schema_graph_json, self._handle
        )

    def subscribe(
        self, object_type: str | None = None, *, actor: dict[str, Any] | None = None
    ) -> Subscription:
        actor_raw = _json_bytes(actor) if actor else b""
        obj_name = object_type.encode() if object_type else None
        sub_handle = self._lib.tesela_runtime_subscribe_json(
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            obj_name,
        )
        if sub_handle == 0:
            raise NativeError(self._last_error())
        return Subscription(self._lib, int(sub_handle))

    def subscribe_changes(
        self, object_type: str, *, actor: dict[str, Any] | None = None
    ) -> Subscription:
        actor_raw = _json_bytes(actor) if actor else b""
        sub_handle = self._lib.tesela_runtime_subscribe_changes_json(
            self._handle,
            _ptr(actor_raw),
            len(actor_raw),
            object_type.encode(),
        )
        if sub_handle == 0:
            raise NativeError(self._last_error())
        return Subscription(self._lib, int(sub_handle))

    def _make_callback(
        self, handler: Callable[[dict[str, Any]], dict[str, Any]]
    ) -> _Callback:
        def invoke(
            _user_data: int, req_ptr: int, req_len: int, out_len: Any
        ) -> int | None:
            try:
                request = json.loads(ctypes.string_at(req_ptr, req_len).decode())
                response = handler(request)
                raw = _json_bytes({"value": response})
            except Exception as exc:
                raw = _json_bytes({"error": {"message": str(exc)}})
            out = _libc.malloc(len(raw))
            if not out:
                out_len[0] = 0
                return None
            ctypes.memmove(out, raw, len(raw))
            out_len[0] = len(raw)
            return out

        return _Callback(invoke)

    def _call_buffer(self, fn: Any, *args: Any) -> dict[str, Any]:
        buf = fn(*args)
        if not buf.data or buf.len <= 0:
            raise NativeError(self._last_error())
        try:
            raw = ctypes.string_at(buf.data, buf.len)
            return json.loads(raw.decode())
        finally:
            self._lib.tesela_buffer_free(buf)

    def _last_error(self) -> str:
        ptr = self._lib.tesela_last_error()
        if not ptr:
            return "unknown native runtime error"
        try:
            return ctypes.string_at(ptr).decode() or "unknown native runtime error"
        finally:
            self._lib.tesela_string_free(ptr)

    def _raise_if_error(self, ptr: int | None) -> None:
        if not ptr:
            return
        try:
            msg = ctypes.string_at(ptr).decode()
        finally:
            self._lib.tesela_string_free(ptr)
        raise NativeError(msg)

    def __enter__(self) -> NativeRuntime:
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass

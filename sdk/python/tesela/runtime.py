"""Native runtime wrapper for the hand-written Tesela Python SDK."""

from __future__ import annotations

import ctypes
import dataclasses
import json
import os
from pathlib import Path
from typing import Any, Callable


class NativeError(RuntimeError):
    """Raised when the native Tesela runtime reports an error."""


@dataclasses.dataclass(frozen=True, slots=True)
class Record:
    """A single ontology record returned by the runtime."""
    _data: dict[str, Any]

    def __getitem__(self, key: str) -> Any:
        return self._data[key]

    def get(self, key: str, default: Any = None) -> Any:
        return self._data.get(key, default)

    def to_dict(self) -> dict[str, Any]:
        return self._data

    def __repr__(self) -> str:
        return f"Record({self._data!r})"


@dataclasses.dataclass(frozen=True, slots=True)
class Page:
    """Paginated record set."""
    records: list[Record]
    next_cursor: str | None = None

    def __getitem__(self, key: str) -> Any:
        return self.to_dict()[key]

    def __len__(self) -> int:
        return len(self.records)

    def __iter__(self):
        return iter(self.records)

    def __bool__(self) -> bool:
        return bool(self.records)

    def to_dict(self) -> dict[str, Any]:
        return {
            "records": [record.to_dict() for record in self.records],
            "next_cursor": self.next_cursor,
        }


@dataclasses.dataclass(frozen=True, slots=True)
class MutationResult:
    """Result of a mutate operation."""
    record: Record | None = None
    rows_affected: int | None = None


@dataclasses.dataclass(frozen=True, slots=True)
class ActionResult:
    """Result of executing an action."""
    value: Any = None

    def to_dict(self) -> dict[str, Any]:
        if isinstance(self.value, dict):
            return self.value
        return {"value": self.value}


@dataclasses.dataclass(frozen=True, slots=True)
class ExplainPlan:
    """Query execution plan."""
    steps: list[dict[str, Any]]


@dataclasses.dataclass(frozen=True, slots=True)
class HealthStatus:
    """Runtime health snapshot."""
    _data: dict[str, Any]

    def __getitem__(self, key: str) -> Any:
        return self._data[key]

    def get(self, key: str, default: Any = None) -> Any:
        return self._data.get(key, default)

    def to_dict(self) -> dict[str, Any]:
        return self._data


class Subscription:
    """Polling-based subscription to domain events or CDC changes."""

    def __init__(self, lib: ctypes.CDLL, sub_handle: int):
        self._lib = lib
        self._handle = sub_handle

    def poll(self, timeout_ms: int = 0) -> dict[str, Any] | None:
        """Poll for the next event.

        * ``timeout_ms=0`` — non-blocking.
        * ``timeout_ms>0`` — block up to N milliseconds.
        * ``timeout_ms=-1`` — block indefinitely.
        """
        buf = self._lib.tesela_runtime_subscribe_poll(self._handle, timeout_ms)
        if not buf.data or buf.len <= 0:
            return None
        try:
            raw = ctypes.string_at(buf.data, buf.len)
            return json.loads(raw.decode())
        finally:
            self._lib.tesela_buffer_free(buf)

    def close(self) -> None:
        self._lib.tesela_runtime_subscribe_close(self._handle)
        self._handle = 0

    def __enter__(self) -> "Subscription":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    def __iter__(self):
        return self

    def __next__(self) -> dict[str, Any]:
        ev = self.poll(timeout_ms=-1)
        if ev is None:
            raise StopIteration
        return ev


class _Buffer(ctypes.Structure):
    _fields_ = [("data", ctypes.c_void_p), ("len", ctypes.c_int)]


_Callback = ctypes.CFUNCTYPE(
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_int,
    ctypes.POINTER(ctypes.c_int),
)


def _wrap_record(data: dict[str, Any] | None) -> Record | None:
    if data is None:
        return None
    return Record(data)


def _wrap_page(data: dict[str, Any]) -> Page:
    records = [Record(r) for r in data.get("records", [])]
    return Page(records=records, next_cursor=data.get("next_cursor"))


def _wrap_mutation(data: dict[str, Any]) -> MutationResult:
    return MutationResult(
        record=_wrap_record(data.get("record")),
        rows_affected=data.get("rows_affected"),
    )


def _wrap_action(data: Any) -> ActionResult:
    return ActionResult(value=data)


def _wrap_explain(data: dict[str, Any]) -> ExplainPlan:
    return ExplainPlan(steps=data.get("steps", []))


def _wrap_health(data: dict[str, Any]) -> HealthStatus:
    return HealthStatus(data)


class NativeRuntime:
    """Handle to the shared Rust runtime loaded through the native ABI."""

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
    ) -> "NativeRuntime":
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
            except (
                Exception
            ) as exc:  # Native side converts invalid JSON into a runtime error.
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

    def __enter__(self) -> "NativeRuntime":
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass


def _load_library(path: str | os.PathLike[str] | None) -> ctypes.CDLL:
    configured_path = path or os.environ.get("TESELA_NATIVE_LIB")
    if configured_path:
        lib_path = Path(configured_path)
    else:
        package_dir = Path(__file__).resolve().parent
        candidates = [
            package_dir / "libtesela_cabi.so",
            package_dir / "libtesela_cabi.dylib",
            package_dir / "tesela_cabi.dll",
        ]
        lib_path = next((c for c in candidates if c.exists()), None)
        if lib_path is None:
            searched = "\n  ".join(str(c) for c in candidates)
            raise OSError(
                f"tesela native library not found. Searched:\n  {searched}\n"
                "Install from a pre-built wheel or set TESELA_NATIVE_LIB to the library path."
            )
    lib = ctypes.CDLL(str(lib_path))
    lib.tesela_runtime_new_from_spec_json.argtypes = [ctypes.c_void_p, ctypes.c_int]
    lib.tesela_runtime_new_from_spec_json.restype = ctypes.c_uint64
    lib.tesela_runtime_release.argtypes = [ctypes.c_uint64]
    lib.tesela_runtime_shutdown.argtypes = [ctypes.c_uint64]
    lib.tesela_runtime_shutdown.restype = ctypes.c_void_p
    lib.tesela_runtime_register_backend.argtypes = [
        ctypes.c_uint64,
        ctypes.c_char_p,
        _Callback,
        ctypes.c_void_p,
    ]
    lib.tesela_runtime_register_backend.restype = ctypes.c_void_p
    lib.tesela_runtime_register_action_handler.argtypes = [
        ctypes.c_uint64,
        ctypes.c_char_p,
        _Callback,
        ctypes.c_void_p,
    ]
    lib.tesela_runtime_register_action_handler.restype = ctypes.c_void_p
    lib.tesela_runtime_register_custom_tool.argtypes = [
        ctypes.c_uint64,
        ctypes.c_char_p,
        _Callback,
        ctypes.c_void_p,
    ]
    lib.tesela_runtime_register_custom_tool.restype = ctypes.c_void_p
    lib.tesela_runtime_register_object_store.argtypes = [
        ctypes.c_uint64,
        ctypes.c_char_p,
        _Callback,
        ctypes.c_void_p,
    ]
    lib.tesela_runtime_register_object_store.restype = ctypes.c_void_p
    lib.tesela_runtime_register_message_bus.argtypes = [
        ctypes.c_uint64,
        ctypes.c_char_p,
        _Callback,
        ctypes.c_void_p,
    ]
    lib.tesela_runtime_register_message_bus.restype = ctypes.c_void_p
    lib.tesela_runtime_register_run_store.argtypes = [
        ctypes.c_uint64,
        ctypes.c_char_p,
        _Callback,
        ctypes.c_void_p,
    ]
    lib.tesela_runtime_register_run_store.restype = ctypes.c_void_p
    lib.tesela_runtime_register_capability_issuer.argtypes = [
        ctypes.c_uint64,
        ctypes.c_char_p,
        _Callback,
        ctypes.c_void_p,
    ]
    lib.tesela_runtime_register_capability_issuer.restype = ctypes.c_void_p
    lib.tesela_runtime_spec_json.argtypes = [ctypes.c_uint64]
    lib.tesela_runtime_spec_json.restype = _Buffer
    lib.tesela_runtime_apply_spec_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_apply_spec_json.restype = _Buffer
    lib.tesela_runtime_search_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_search_json.restype = _Buffer
    lib.tesela_runtime_get_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_get_json.restype = _Buffer
    lib.tesela_runtime_mutate_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_mutate_json.restype = _Buffer
    lib.tesela_runtime_execute_action_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_execute_action_json.restype = _Buffer
    lib.tesela_runtime_explain_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_explain_json.restype = _Buffer
    lib.tesela_runtime_traverse_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_traverse_json.restype = _Buffer
    lib.tesela_runtime_aggregate_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_aggregate_json.restype = _Buffer
    lib.tesela_runtime_aggregate_view_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
    ]
    lib.tesela_runtime_aggregate_view_json.restype = _Buffer
    lib.tesela_runtime_issue_capability_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_issue_capability_json.restype = _Buffer
    lib.tesela_runtime_authorize_artifact_read_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_uint64,
    ]
    lib.tesela_runtime_authorize_artifact_read_json.restype = _Buffer
    lib.tesela_runtime_initiate_upload_flow_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_uint64,
    ]
    lib.tesela_runtime_initiate_upload_flow_json.restype = _Buffer
    lib.tesela_runtime_start_job_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_start_job_json.restype = _Buffer
    lib.tesela_runtime_complete_upload_flow_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_complete_upload_flow_json.restype = _Buffer
    lib.tesela_runtime_load_upload_flow_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_load_upload_flow_json.restype = _Buffer
    lib.tesela_runtime_upload_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_upload_json.restype = _Buffer
    lib.tesela_runtime_rollback_upload_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_rollback_upload_json.restype = _Buffer
    lib.tesela_runtime_agent_start_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_agent_start_json.restype = _Buffer
    lib.tesela_runtime_agent_get_run_json.argtypes = [ctypes.c_uint64, ctypes.c_char_p]
    lib.tesela_runtime_agent_get_run_json.restype = _Buffer
    lib.tesela_runtime_health_json.argtypes = [ctypes.c_uint64]
    lib.tesela_runtime_health_json.restype = _Buffer
    lib.tesela_runtime_capabilities_json.argtypes = [ctypes.c_uint64]
    lib.tesela_runtime_capabilities_json.restype = _Buffer
    lib.tesela_runtime_vector_search_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_vector_search_json.restype = _Buffer
    lib.tesela_runtime_resolve_object_set_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
    ]
    lib.tesela_runtime_resolve_object_set_json.restype = _Buffer
    lib.tesela_runtime_compose_object_sets_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_compose_object_sets_json.restype = _Buffer
    lib.tesela_runtime_execute_pipeline_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_execute_pipeline_json.restype = _Buffer
    lib.tesela_runtime_get_lineage_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_int,
    ]
    lib.tesela_runtime_get_lineage_json.restype = _Buffer
    lib.tesela_runtime_cross_search_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_cross_search_json.restype = _Buffer
    lib.tesela_runtime_create_branch_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
    ]
    lib.tesela_runtime_create_branch_json.restype = _Buffer
    lib.tesela_runtime_update_branch_spec_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_update_branch_spec_json.restype = _Buffer
    lib.tesela_runtime_merge_branch_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
    ]
    lib.tesela_runtime_merge_branch_json.restype = _Buffer
    lib.tesela_runtime_list_branches_json.argtypes = [ctypes.c_uint64]
    lib.tesela_runtime_list_branches_json.restype = _Buffer
    lib.tesela_runtime_apply_spec_with_migration_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
    ]
    lib.tesela_runtime_apply_spec_with_migration_json.restype = _Buffer
    lib.tesela_runtime_schema_graph_json.argtypes = [ctypes.c_uint64]
    lib.tesela_runtime_schema_graph_json.restype = _Buffer
    lib.tesela_runtime_subscribe_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
    ]
    lib.tesela_runtime_subscribe_json.restype = ctypes.c_uint64
    lib.tesela_runtime_subscribe_changes_json.argtypes = [
        ctypes.c_uint64,
        ctypes.c_void_p,
        ctypes.c_int,
        ctypes.c_char_p,
    ]
    lib.tesela_runtime_subscribe_changes_json.restype = ctypes.c_uint64
    lib.tesela_runtime_subscribe_poll.argtypes = [
        ctypes.c_uint64,
        ctypes.c_int,
    ]
    lib.tesela_runtime_subscribe_poll.restype = _Buffer
    lib.tesela_runtime_subscribe_close.argtypes = [ctypes.c_uint64]
    lib.tesela_last_error.restype = ctypes.c_void_p
    lib.tesela_string_free.argtypes = [ctypes.c_void_p]
    lib.tesela_buffer_free.argtypes = [_Buffer]
    return lib


def _json_bytes(value: dict[str, Any] | str | bytes | Any) -> bytes:
    if isinstance(value, bytes):
        return value
    if isinstance(value, str):
        return value.encode()
    return json.dumps(value, separators=(",", ":")).encode()


def _ptr(raw: bytes) -> ctypes.c_char_p | None:
    if not raw:
        return None
    return ctypes.c_char_p(raw)


_libc = ctypes.CDLL(None)
_libc.malloc.argtypes = [ctypes.c_size_t]
_libc.malloc.restype = ctypes.c_void_p


# ------------------------------------------------------------------
# Async wrapper (Python 3.9+ asyncio.to_thread)
# ------------------------------------------------------------------

import asyncio


class AsyncNativeRuntime:
    """Async wrapper around ``NativeRuntime`` that offloads every call
    to a background thread via ``asyncio.to_thread``.
    """

    def __init__(self, runtime: NativeRuntime):
        self._rt = runtime

    @classmethod
    async def from_app(
        cls, app: Any, *, library_path: str | os.PathLike[str] | None = None
    ) -> "AsyncNativeRuntime":
        loop = asyncio.get_running_loop()
        rt = await loop.run_in_executor(None, NativeRuntime.from_app, app, library_path)
        return cls(rt)

    @classmethod
    async def from_spec(
        cls,
        spec: dict[str, Any] | str | bytes,
        *,
        library_path: str | os.PathLike[str] | None = None,
    ) -> "AsyncNativeRuntime":
        loop = asyncio.get_running_loop()
        rt = await loop.run_in_executor(None, lambda: NativeRuntime(spec, library_path=library_path))
        return cls(rt)

    async def close(self) -> None:
        await asyncio.to_thread(self._rt.close)

    async def shutdown(self) -> None:
        await asyncio.to_thread(self._rt.shutdown)

    async def spec(self) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.spec)

    async def apply_spec(self, spec: dict[str, Any] | str | bytes) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.apply_spec, spec)

    async def register_backend(
        self, adapter_type: str, handler: Callable[[dict[str, Any]], dict[str, Any]]
    ) -> None:
        return await asyncio.to_thread(self._rt.register_backend, adapter_type, handler)

    async def register_action_handler(
        self, kind: str, handler: Callable[[dict[str, Any]], dict[str, Any]]
    ) -> None:
        return await asyncio.to_thread(self._rt.register_action_handler, kind, handler)

    async def register_custom_tool(
        self, name: str, handler: Callable[[dict[str, Any]], dict[str, Any]]
    ) -> None:
        return await asyncio.to_thread(self._rt.register_custom_tool, name, handler)

    async def search(
        self, object_type: str, query: dict[str, Any] | None = None, *, actor: dict[str, Any] | None = None
    ) -> Page:
        return await asyncio.to_thread(self._rt.search, object_type, query, actor=actor)

    async def get(
        self, object_type: str, primary_key: Any, *, actor: dict[str, Any] | None = None
    ) -> Record | None:
        return await asyncio.to_thread(self._rt.get, object_type, primary_key, actor=actor)

    async def mutate(
        self, object_type: str, mutation: dict[str, Any], *, actor: dict[str, Any] | None = None
    ) -> MutationResult:
        return await asyncio.to_thread(self._rt.mutate, object_type, mutation, actor=actor)

    async def execute_action(
        self, action: str, input: dict[str, Any] | None = None, *, actor: dict[str, Any] | None = None
    ) -> ActionResult:
        return await asyncio.to_thread(self._rt.execute_action, action, input, actor=actor)

    async def explain(
        self, object_type: str, query: dict[str, Any] | None = None, *, actor: dict[str, Any] | None = None
    ) -> ExplainPlan:
        return await asyncio.to_thread(self._rt.explain, object_type, query, actor=actor)

    async def traverse(
        self, link_type: str, query: dict[str, Any] | None = None, *, actor: dict[str, Any] | None = None
    ) -> Page:
        return await asyncio.to_thread(self._rt.traverse, link_type, query, actor=actor)

    async def aggregate(
        self, object_type: str, query: dict[str, Any] | None = None, *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.aggregate, object_type, query, actor=actor)

    async def upload(
        self, object_type: str, content: bytes | str, *, format: str = "csv",
        mappings: list[dict[str, Any]] | None = None, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(
            self._rt.upload, object_type, content, format=format, mappings=mappings, actor=actor
        )

    async def rollback_upload(
        self, object_type: str, load_id: str, *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.rollback_upload, object_type, load_id, actor=actor)

    async def agent_start(
        self, agent: str, input: dict[str, Any] | None = None, *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.agent_start, agent, input, actor=actor)

    async def agent_get_run(self, run_id: str) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.agent_get_run, run_id)

    async def health(self) -> HealthStatus:
        return await asyncio.to_thread(self._rt.health)

    async def capabilities(self) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.capabilities)

    async def vector_search(
        self, query: dict[str, Any], *, actor: dict[str, Any] | None = None
    ) -> Page:
        return await asyncio.to_thread(self._rt.vector_search, query, actor=actor)

    async def resolve_object_set(
        self, name: str, *, actor: dict[str, Any] | None = None
    ) -> Page:
        return await asyncio.to_thread(self._rt.resolve_object_set, name, actor=actor)

    async def compose_object_sets(
        self, names: list[str], op: str = "union", *, actor: dict[str, Any] | None = None
    ) -> Page:
        return await asyncio.to_thread(self._rt.compose_object_sets, names, op, actor=actor)

    async def execute_pipeline(
        self, pipeline_name: str, mode: str = "incremental", *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.execute_pipeline, pipeline_name, mode, actor=actor)

    async def get_lineage(
        self, object_type: str, pk: Any, *, depth: int | None = None, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.get_lineage, object_type, pk, depth=depth, actor=actor)

    async def cross_search(
        self, queries: list[dict[str, Any]], *, actor: dict[str, Any] | None = None
    ) -> Page:
        return await asyncio.to_thread(self._rt.cross_search, queries, actor=actor)

    async def create_branch(
        self, display: str, *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.create_branch, display, actor=actor)

    async def update_branch_spec(
        self, branch_id: str, spec: dict[str, Any] | str | bytes, *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.update_branch_spec, branch_id, spec, actor=actor)

    async def merge_branch(
        self, branch_id: str, *, actor: dict[str, Any] | None = None
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.merge_branch, branch_id, actor=actor)

    async def list_branches(self) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.list_branches)

    async def apply_spec_with_migration(
        self, spec: dict[str, Any] | str | bytes
    ) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.apply_spec_with_migration, spec)

    async def schema_graph(self) -> dict[str, Any]:
        return await asyncio.to_thread(self._rt.schema_graph)

    async def subscribe(
        self, object_type: str | None = None, *, actor: dict[str, Any] | None = None
    ) -> Subscription:
        return await asyncio.to_thread(self._rt.subscribe, object_type, actor=actor)

    async def subscribe_changes(
        self, object_type: str, *, actor: dict[str, Any] | None = None
    ) -> Subscription:
        return await asyncio.to_thread(self._rt.subscribe_changes, object_type, actor=actor)

    def __repr__(self) -> str:
        return f"AsyncNativeRuntime(handle={self._rt._handle})"

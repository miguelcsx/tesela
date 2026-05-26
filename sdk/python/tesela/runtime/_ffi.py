from __future__ import annotations

import ctypes
import json
import os
from pathlib import Path
from typing import Any


class _Buffer(ctypes.Structure):
    _fields_ = [("data", ctypes.c_void_p), ("len", ctypes.c_int)]


_Callback = ctypes.CFUNCTYPE(
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_void_p,
    ctypes.c_int,
    ctypes.POINTER(ctypes.c_int),
)

_c = ctypes

_HANDLE_ONLY_BUFFER = [_c.c_uint64]

_ACTOR_CHARPTR_BUFFER = [
    _c.c_uint64, _c.c_void_p, _c.c_int, _c.c_char_p,
]

_ACTOR_CHARPTR_BLOB_BUFFER = [
    _c.c_uint64, _c.c_void_p, _c.c_int, _c.c_char_p, _c.c_void_p, _c.c_int,
]

_ACTOR_BLOB_BUFFER = [
    _c.c_uint64, _c.c_void_p, _c.c_int, _c.c_void_p, _c.c_int,
]

_ACTOR_CHARPTR_BLOB_U64_BUFFER = [
    _c.c_uint64, _c.c_void_p, _c.c_int, _c.c_char_p, _c.c_void_p, _c.c_int, _c.c_uint64,
]

_ACTOR_CHARPTR_BLOB_INT_BUFFER = [
    _c.c_uint64, _c.c_void_p, _c.c_int, _c.c_char_p, _c.c_void_p, _c.c_int, _c.c_int,
]

_BUFFER_SIGS: dict[str, list] = {
    "tesela_runtime_spec_json": _HANDLE_ONLY_BUFFER,
    "tesela_runtime_apply_spec_json": [_c.c_uint64, _c.c_void_p, _c.c_int],
    "tesela_runtime_search_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_get_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_mutate_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_execute_action_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_explain_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_traverse_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_aggregate_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_aggregate_view_json": _ACTOR_CHARPTR_BUFFER,
    "tesela_runtime_issue_capability_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_authorize_artifact_read_json": _ACTOR_CHARPTR_BLOB_U64_BUFFER,
    "tesela_runtime_initiate_upload_flow_json": _ACTOR_CHARPTR_BLOB_U64_BUFFER,
    "tesela_runtime_start_job_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_complete_upload_flow_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_load_upload_flow_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_upload_json": [
        _c.c_uint64, _c.c_void_p, _c.c_int, _c.c_char_p,
        _c.c_void_p, _c.c_int, _c.c_void_p, _c.c_int,
    ],
    "tesela_runtime_rollback_upload_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_agent_start_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_agent_get_run_json": [_c.c_uint64, _c.c_char_p],
    "tesela_runtime_configure_apxm_agent_runtime_json": [_c.c_uint64, _c.c_void_p, _c.c_int],
    "tesela_runtime_health_json": _HANDLE_ONLY_BUFFER,
    "tesela_runtime_capabilities_json": _HANDLE_ONLY_BUFFER,
    "tesela_runtime_vector_search_json": _ACTOR_BLOB_BUFFER,
    "tesela_runtime_resolve_object_set_json": _ACTOR_CHARPTR_BUFFER,
    "tesela_runtime_compose_object_sets_json": _ACTOR_BLOB_BUFFER,
    "tesela_runtime_execute_pipeline_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_get_lineage_json": _ACTOR_CHARPTR_BLOB_INT_BUFFER,
    "tesela_runtime_cross_search_json": _ACTOR_BLOB_BUFFER,
    "tesela_runtime_create_branch_json": _ACTOR_CHARPTR_BUFFER,
    "tesela_runtime_update_branch_spec_json": _ACTOR_CHARPTR_BLOB_BUFFER,
    "tesela_runtime_merge_branch_json": _ACTOR_CHARPTR_BUFFER,
    "tesela_runtime_list_branches_json": _HANDLE_ONLY_BUFFER,
    "tesela_runtime_apply_spec_with_migration_json": [_c.c_uint64, _c.c_void_p, _c.c_int],
    "tesela_runtime_schema_graph_json": _HANDLE_ONLY_BUFFER,
    "tesela_runtime_add_entity_json": [_c.c_uint64, _c.c_char_p, _c.c_void_p, _c.c_int],
    "tesela_runtime_remove_entity_json": [_c.c_uint64, _c.c_char_p, _c.c_char_p],
    "tesela_runtime_subscribe_poll": [_c.c_uint64, _c.c_int],
}

_VOID_PTR_SIGS: dict[str, list] = {
    "tesela_runtime_shutdown": [_c.c_uint64],
    "tesela_runtime_register_backend": [_c.c_uint64, _c.c_char_p, _Callback, _c.c_void_p],
    "tesela_runtime_register_action_handler": [_c.c_uint64, _c.c_char_p, _Callback, _c.c_void_p],
    "tesela_runtime_register_custom_tool": [_c.c_uint64, _c.c_char_p, _Callback, _c.c_void_p],
    "tesela_runtime_register_object_store": [_c.c_uint64, _c.c_char_p, _Callback, _c.c_void_p],
    "tesela_runtime_register_message_bus": [_c.c_uint64, _c.c_char_p, _Callback, _c.c_void_p],
    "tesela_runtime_register_run_store": [_c.c_uint64, _c.c_char_p, _Callback, _c.c_void_p],
    "tesela_runtime_register_capability_issuer": [_c.c_uint64, _c.c_char_p, _Callback, _c.c_void_p],
}

_U64_SIGS: dict[str, list] = {
    "tesela_runtime_subscribe_json": _ACTOR_CHARPTR_BUFFER,
    "tesela_runtime_subscribe_changes_json": _ACTOR_CHARPTR_BUFFER,
}

_MISC_SIGS: list[tuple[str, list, Any]] = [
    ("tesela_runtime_new_from_spec_json", [_c.c_void_p, _c.c_int], _c.c_uint64),
    ("tesela_runtime_release", [_c.c_uint64], None),
    ("tesela_last_error", [], _c.c_void_p),
    ("tesela_string_free", [_c.c_void_p], None),
    ("tesela_buffer_free", [_Buffer], None),
    ("tesela_runtime_subscribe_close", [_c.c_uint64], None),
]


def _load_library(path: str | os.PathLike[str] | None) -> _c.CDLL:
    configured_path = path or os.environ.get("TESELA_NATIVE_LIB")
    if configured_path:
        lib_path = Path(configured_path)
    else:
        package_dir = Path(__file__).resolve().parent.parent
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
    lib = _c.CDLL(str(lib_path))

    for name, argtypes in _BUFFER_SIGS.items():
        fn = getattr(lib, name)
        fn.argtypes = argtypes
        fn.restype = _Buffer

    for name, argtypes in _VOID_PTR_SIGS.items():
        fn = getattr(lib, name)
        fn.argtypes = argtypes
        fn.restype = _c.c_void_p

    for name, argtypes in _U64_SIGS.items():
        fn = getattr(lib, name)
        fn.argtypes = argtypes
        fn.restype = _c.c_uint64

    for name, argtypes, restype in _MISC_SIGS:
        fn = getattr(lib, name)
        if argtypes:
            fn.argtypes = argtypes
        if restype is not None:
            fn.restype = restype

    return lib


def _json_bytes(value: dict[str, Any] | str | bytes | Any) -> bytes:
    if isinstance(value, bytes):
        return value
    if isinstance(value, str):
        return value.encode()
    return json.dumps(value, separators=(",", ":")).encode()


def _ptr(raw: bytes) -> _c.c_char_p | None:
    if not raw:
        return None
    return _c.c_char_p(raw)


_libc = _c.CDLL(None)
_libc.malloc.argtypes = [_c.c_size_t]
_libc.malloc.restype = _c.c_void_p

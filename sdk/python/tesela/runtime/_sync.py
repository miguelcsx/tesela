from __future__ import annotations

import json
from typing import Any, Callable

from tesela._native import NativeRuntime as _NativeRuntime
from tesela.runtime._types import (
    ActionResult,
    HealthStatus,
    NativeError,
    Page,
    Record,
    _wrap_action,
    _wrap_explain,
    _wrap_health,
    _wrap_mutation,
    _wrap_page,
    _wrap_record,
)


class NativeRuntime:
    def __init__(self, spec: dict[str, Any] | str | bytes):
        self._native = _NativeRuntime(_encode(spec))

    @classmethod
    def from_app(cls, app: Any) -> NativeRuntime:
        compile_spec = getattr(app, "compile", None)
        if not callable(compile_spec):
            raise TypeError("app must expose compile()")
        return cls(compile_spec())

    @classmethod
    def from_spec(cls, spec: dict[str, Any] | str | bytes) -> NativeRuntime:
        return cls(spec)

    @property
    def spec(self) -> dict[str, Any]:
        return json.loads(self._native.spec_json())

    def apply_spec(self, spec: dict[str, Any] | str | bytes) -> dict[str, Any]:
        return _decode(self._native.apply_spec_json(_encode(spec)))

    def register_backend(self, adapter_type: str, handler: Callable[[dict[str, Any]], Any]) -> None:
        if not callable(handler):
            raise TypeError("handler must be callable")
        self._native.register_backend(adapter_type, handler)

    def search(self, object_type: str, query: dict[str, Any] | None = None) -> Page:
        raw = self._native.search_json(object_type, _encode(query or {}), None)
        return _wrap_page(_decode(raw))

    def get(self, object_type: str, primary_key: str) -> Record | None:
        raw = self._native.get_json(object_type, _encode(primary_key), None)
        return _wrap_record(_decode(raw))

    def mutate(self, object_type: str, op: dict[str, Any]) -> Any:
        raw = self._native.mutate_json(object_type, _encode(op), None)
        return _wrap_mutation(_decode(raw))

    def execute_action(self, action: str, input: dict[str, Any] | None = None) -> ActionResult:
        raw = self._native.execute_action_json(action, _encode(input or {}), None)
        return _wrap_action(_decode(raw))

    def explain(self, object_type: str, query: dict[str, Any]) -> Any:
        raw = self._native.explain_json(object_type, _encode(query), None)
        return _wrap_explain(_decode(raw))

    def traverse(self, link_type: str, query: dict[str, Any]) -> dict[str, Any]:
        return _decode(self._native.traverse_json(link_type, _encode(query), None))

    def aggregate(self, object_type: str, query: dict[str, Any]) -> dict[str, Any]:
        return _decode(self._native.aggregate_json(object_type, _encode(query), None))

    def aggregate_view(self, view: str) -> dict[str, Any]:
        return _decode(self._native.aggregate_view_json(view, None))

    def health(self) -> HealthStatus:
        return _wrap_health(_decode(self._native.health_json()))

    def capabilities(self) -> dict[str, Any]:
        return _decode(self._native.capabilities_json())

    def add_entity(self, kind: str, entity: dict[str, Any]) -> Any:
        return _decode(self._native.add_entity_json(kind, _encode(entity)))

    def close(self) -> None:
        pass

    def __enter__(self) -> NativeRuntime:
        return self

    def __exit__(self, *_: Any) -> None:
        self.close()

    def __repr__(self) -> str:
        return "NativeRuntime()"


def _encode(value: Any) -> str:
    if isinstance(value, bytes):
        return value.decode()
    if isinstance(value, str):
        return value
    return json.dumps(value)


def _decode(value: str) -> Any:
    try:
        return json.loads(value)
    except ValueError as exc:
        raise NativeError(str(exc)) from exc

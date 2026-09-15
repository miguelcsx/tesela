"""Declarative Tesela ontology and native runtime."""

from __future__ import annotations

import inspect
import json
import re
from copy import deepcopy
from typing import Any, get_args, get_origin, get_type_hints

from . import _native

MemoryStore = _native.NativeMemoryStore

_SECTIONS = {
    "datasources", "traits", "object_types", "link_types", "actions",
    "roles", "policies", "object_sets",
}
_NAME = re.compile(r"^[a-z][a-z0-9_]*$")
_STORE_ARGUMENTS = {
    "search": ("object_type", "query"),
    "get": ("object_type", "primary_key"),
    "create": ("object_type", "values"),
    "update": ("object_type", "primary_key", "values"),
    "delete": ("object_type", "primary_key"),
    "aggregate": ("object_type", "query"),
    "traverse": ("link_type", "query"),
    "execute_action": ("request",),
}


def _name(value: str) -> str:
    if not _NAME.fullmatch(value):
        raise ValueError(f"invalid api_name {value!r}; use lowercase letters, digits and underscores")
    return value


def _snake(value: str) -> str:
    return _name("".join(("_" if index and char.isupper() else "") + char.lower() for index, char in enumerate(value)))


def _type(annotation: Any) -> tuple[str, bool]:
    nullable = type(None) in get_args(annotation)
    if nullable:
        annotation = next(item for item in get_args(annotation) if item is not type(None))
    types = {str: "string", int: "integer", float: "float", bool: "boolean", dict: "json", list: "array"}
    if annotation in types:
        return types[annotation], nullable
    if get_origin(annotation) in (list, dict):
        return types[get_origin(annotation)], nullable
    raise TypeError(f"unsupported property annotation {annotation!r}; supply a raw property definition")


def _properties(cls: type) -> list[dict[str, Any]]:
    properties = []
    for name, annotation in get_type_hints(cls).items():
        data_type, nullable = _type(annotation)
        property_ = {"api_name": _name(name), "data_type": data_type}
        if nullable:
            property_["nullable"] = True
        properties.append(property_)
    return properties


def _check_fields(original: Any, normalized: Any, path: str = "spec") -> None:
    if isinstance(original, dict) and isinstance(normalized, dict):
        for key, value in original.items():
            location = f"{path}.{key}"
            if key not in normalized:
                if value not in (None, [], {}):
                    raise ValueError(f"unsupported Spec field {location}")
            else:
                _check_fields(value, normalized[key], location)
    elif isinstance(original, list) and isinstance(normalized, list):
        for index, (item, result) in enumerate(zip(original, normalized)):
            _check_fields(item, result, f"{path}[{index}]")


class Spec:
    """Canonical `tesela.spec.v1` document with concise decorators."""

    def __init__(self, workspace: str = "default", *, data: dict[str, Any] | None = None):
        self._data = deepcopy(data) if data is not None else {"version": "tesela.spec.v1", "workspace": {"api_name": _name(workspace)}}

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> Spec:
        return cls(data=data)

    @classmethod
    def from_json(cls, data: str) -> Spec:
        return cls.from_dict(json.loads(data))

    def to_dict(self) -> dict[str, Any]:
        return deepcopy(self._data)

    def to_json(self) -> str:
        data = json.dumps(self._data)
        _check_fields(self._data, json.loads(_native.normalize_spec(data)))
        return data

    def add(self, section: str, item: dict[str, Any]) -> None:
        if section not in _SECTIONS:
            raise ValueError(f"unknown Spec section {section!r}")
        name = _name(item["api_name"])
        entries = self._data.setdefault(section, [])
        entries[:] = [existing for existing in entries if existing["api_name"] != name]
        entries.append(deepcopy(item))

    def datasource(self, name: str, adapter_type: str = "memory", **fields: Any) -> None:
        self.add("datasources", {"api_name": _name(name), "adapter_type": adapter_type, **fields})

    def object_type(self, *, datasource: str, primary_key: str, name: str | None = None, **fields: Any):
        def decorator(cls: type) -> type:
            api_name = _name(name) if name is not None else _snake(cls.__name__)
            properties = _properties(cls)
            if primary_key not in {item["api_name"] for item in properties}:
                raise ValueError(f"primary key {primary_key!r} is not an annotated property")
            self.add("object_types", {"api_name": api_name, "source": {"datasource": _name(datasource), "resource": api_name}, "primary_key": _name(primary_key), "properties": properties, **fields})
            return cls
        return decorator

    def trait(self, cls: type | None = None, *, name: str | None = None, **fields: Any):
        def decorator(value: type) -> type:
            self.add("traits", {"api_name": _name(name) if name is not None else _snake(value.__name__), "display": value.__name__, "properties": _properties(value), **fields})
            return value
        return decorator(cls) if cls is not None else decorator

    def link(self, *, from_type: str, to_type: str, cardinality: str = "one_to_many", name: str | None = None, **fields: Any):
        def decorator(cls: type) -> type:
            self.add("link_types", {"api_name": _name(name) if name is not None else _snake(cls.__name__), "display": cls.__name__, "from": _name(from_type), "to": _name(to_type), "cardinality": cardinality, **fields})
            return cls
        return decorator

    def action(self, *, subject: str | None = None, name: str | None = None, **fields: Any):
        def decorator(fn):
            props = {}
            hints = get_type_hints(fn)
            for parameter in inspect.signature(fn).parameters.values():
                if parameter.name not in hints:
                    raise TypeError(f"action parameter {parameter.name!r} needs a type annotation")
                data_type, _ = _type(hints[parameter.name])
                props[parameter.name] = {"type": {"string": "string", "integer": "integer", "float": "number", "boolean": "boolean", "json": "object", "array": "array"}[data_type]}
            action = {"api_name": _name(name or fn.__name__), "display": fn.__name__, "handler": {"kind": "callback", "target": fn.__name__}, "risk_level": "low"}
            if props:
                action["input_schema"] = {"type": "object", "properties": props}
            if subject is not None:
                action["subject"] = _name(subject)
            action.update(fields)
            self.add("actions", action)
            return fn
        return decorator

    def policy(self, *, effect: str = "allow", roles: list[str] | None = None, operations: list[str] | None = None, name: str | None = None, **fields: Any):
        def decorator(fn):
            self.add("policies", {"api_name": _name(name or fn.__name__), "effect": effect, "roles": roles or [], "operations": operations or [], **fields})
            return fn
        return decorator


class _StoreAdapter:
    def __init__(self, store: Any):
        self.store = store

    def capabilities(self, _: str) -> str:
        supported = {name: callable(getattr(self.store, name, None)) for name in _STORE_ARGUMENTS}
        return json.dumps(supported)

    def __getattr__(self, name: str):
        if name not in _STORE_ARGUMENTS:
            raise AttributeError(name)
        def invoke(payload: str) -> str:
            payload_data = json.loads(payload)
            return json.dumps(getattr(self.store, name)(*(payload_data[key] for key in _STORE_ARGUMENTS[name])))
        return invoke


class _PolicyAdapter:
    def __init__(self, port: Any):
        self.port = port

    def evaluate(self, payload: str) -> str:
        return json.dumps(self.port.evaluate(json.loads(payload)["request"]))


class _AuditAdapter:
    def __init__(self, port: Any):
        self.port = port

    def record(self, payload: str) -> str:
        self.port.record(json.loads(payload)["event"])
        return "null"


class _EventAdapter:
    def __init__(self, port: Any):
        self.port = port

    def publish(self, payload: str) -> str:
        self.port.publish(json.loads(payload)["event"])
        return "null"


class Runtime:
    """Native runtime; pass an explicit policy and datasource-to-store mapping."""

    def __init__(self, spec: Spec, *, stores: dict[str, Any], policy: Any, audit: Any = None, events: Any = None, max_query_limit: int | None = None):
        for name, store in stores.items():
            if not isinstance(store, MemoryStore):
                missing = [method for method in ("search", "get", "create", "update", "delete") if not callable(getattr(store, method, None))]
                if missing:
                    raise TypeError(f"store {name!r} is missing required methods: {', '.join(missing)}")
        if policy is None or (not isinstance(policy, str) and not callable(getattr(policy, "evaluate", None))):
            raise TypeError("policy must implement evaluate(request), or be 'allow_all' for local development")
        adapted = {name: store if isinstance(store, MemoryStore) else _StoreAdapter(store) for name, store in stores.items()}
        if isinstance(policy, str) and policy != "allow_all":
            raise ValueError("policy must be an object with evaluate(request), or 'allow_all' for local development")
        if audit is not None and not callable(getattr(audit, "record", None)):
            raise TypeError("audit must implement record(event)")
        if events is not None and not callable(getattr(events, "publish", None)):
            raise TypeError("events must implement publish(event)")
        dev_policy = isinstance(policy, str) and policy == "allow_all"
        self._native = _native.NativeRuntime(spec.to_json(), adapted, policy if dev_policy else _PolicyAdapter(policy), _AuditAdapter(audit) if audit is not None else None, _EventAdapter(events) if events is not None else None, max_query_limit)

    def spec(self) -> Spec:
        return Spec.from_json(self._native.spec())

    def apply_spec(self, spec: Spec) -> None:
        self._native.apply_spec(spec.to_json())

    def search(self, object_type: str, *, actor: dict[str, Any], query: dict[str, Any] | None = None) -> dict[str, Any]:
        return json.loads(self._native.search(json.dumps(actor), object_type, json.dumps(query or {})))

    def get(self, object_type: str, primary_key: Any, *, actor: dict[str, Any]) -> dict[str, Any]:
        return json.loads(self._native.get(json.dumps(actor), object_type, json.dumps(primary_key)))

    def mutate(self, object_type: str, mutation: dict[str, Any], *, actor: dict[str, Any]) -> dict[str, Any]:
        return json.loads(self._native.mutate(json.dumps(actor), object_type, json.dumps(mutation)))

    def aggregate(self, object_type: str, *, actor: dict[str, Any], query: dict[str, Any] | None = None) -> dict[str, Any]:
        return json.loads(self._native.aggregate(json.dumps(actor), object_type, json.dumps(query or {})))

    def traverse(self, link_type: str, query: dict[str, Any], *, actor: dict[str, Any]) -> dict[str, Any]:
        return json.loads(self._native.traverse(json.dumps(actor), link_type, json.dumps(query)))

    def resolve_object_set(self, name: str, *, actor: dict[str, Any]) -> dict[str, Any]:
        return json.loads(self._native.resolve_object_set(json.dumps(actor), name))

    def compose_object_sets(self, names: list[str], op: str, *, actor: dict[str, Any]) -> dict[str, Any]:
        return json.loads(self._native.compose_object_sets(json.dumps(actor), json.dumps(names), json.dumps(op)))

    def execute_action(self, action: str, input: Any, *, actor: dict[str, Any], run_id: str | None = None) -> dict[str, Any]:
        request = {"action": action, "input": input, "actor": actor}
        if run_id is not None:
            request["run_id"] = run_id
        return json.loads(self._native.execute_action(json.dumps(request)))


def tool_definitions() -> list[dict[str, Any]]:
    return json.loads(_native.tool_definitions())


__all__ = ["Spec", "Runtime", "MemoryStore", "tool_definitions"]

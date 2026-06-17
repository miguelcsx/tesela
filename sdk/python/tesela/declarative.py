"""Declarative API — minimal, pythonic, readable.

Decorator-first ontology definitions. Use as ``@app.entity(datasource="memory")``
or standalone ``@entity(app, ...)``.
"""

from __future__ import annotations

import dataclasses
import inspect
import typing
import json
from typing import Any

_PY_TO_TESELA = {str: "string", int: "integer", float: "float", bool: "boolean", list: "array"}


def _type_map(t: Any) -> str:
    args = typing.get_args(t)
    if args and type(None) in args:
        t = next(a for a in args if a is not type(None))
    return _PY_TO_TESELA.get(t, "string")


def _entity(app: "App", cls: Any, *, datasource: str = "memory",
            resource: str = "", primary_key: str = "id") -> Any:
    if not dataclasses.is_dataclass(cls):
        cls = dataclasses.dataclass(cls)

    hints = typing.get_type_hints(cls)
    props = []
    for f in dataclasses.fields(cls):
        dt = _type_map(hints.get(f.name, str))
        meta = f.metadata
        p: dict[str, Any] = {
            "api_name": f.name,
            "data_type": dt,
            "nullable": meta.get("nullable", False),
            "indexed": bool(meta.get("indexed")),
            "unique": bool(meta.get("unique")),
        }
        if meta.get("description"):
            p["description"] = meta["description"]
        if meta.get("source_column"):
            p["source_column"] = meta["source_column"]
        if meta.get("encrypted"):
            p["encrypted"] = True
        if meta.get("default") is not None:
            p["default"] = meta["default"]
        props.append(p)

    src: dict[str, Any] = {"datasource": datasource}
    if resource:
        src["resource"] = resource

    app._entities.append({
        "api_name": cls.__name__.lower(),
        "display": cls.__name__,
        "source": src,
        "primary_key": primary_key,
        "properties": props,
    })
    return cls


def _link(app: "App", cls: Any, *, from_type: str = "", to_type: str = "",
          cardinality: str = "one_to_many") -> Any:
    app._links.append({
        "api_name": cls.__name__.lower(),
        "display": cls.__name__,
        "from": from_type or getattr(cls, "FROM", ""),
        "to": to_type or getattr(cls, "TO", ""),
        "cardinality": cardinality or getattr(cls, "CARD", "one_to_many"),
        "mappings": getattr(cls, "MAPPINGS", []),
    })
    return cls


def _action(app: "App", fn: Any, *, description: str = "", risk_level: str = "low",
            mode: str = "", handler_kind: str = "callback") -> Any:
    sig = inspect.signature(fn)
    hints = typing.get_type_hints(fn)
    props = {n: {"type": _type_map(hints.get(n, str))} for n in sig.parameters}

    a: dict[str, Any] = {
        "api_name": fn.__name__,
        "description": description or fn.__doc__ or "",
        "risk_level": risk_level,
        "handler": {"kind": handler_kind, "target": fn.__name__},
        "input_schema": {"type": "object", "properties": props},
    }
    if mode:
        a["mode"] = mode
    app._actions.append(a)
    return fn


def _agent(app: "App", cls: Any, *, model: str = "", allowed_tools: list[str] | None = None) -> Any:
    app._agents.append({
        "api_name": cls.__name__.lower(),
        "display": cls.__name__,
        "model": model or getattr(cls, "MODEL", "claude-sonnet-4-6"),
        "instructions": getattr(cls, "__doc__", ""),
        "allowed_tools": allowed_tools or getattr(cls, "tools", []),
    })
    return cls


def _policy(app: "App", target: Any, *, effect: str = "allow",
            roles: list[str] | None = None, operations: list[str] | None = None,
            resource_kind: str = "", resource: str = "") -> Any:
    if callable(target) and not isinstance(target, type):
        api_name = target.__name__
        desc = target.__doc__ or ""
    else:
        api_name = target.__name__.lower()
        effect = getattr(target, "EFFECT", effect)
        roles = getattr(target, "ROLES", roles)
        operations = getattr(target, "OPERATIONS", operations)
        resource_kind = getattr(target, "RESOURCE_KIND", resource_kind)
        resource = getattr(target, "RESOURCE", resource)
        desc = getattr(target, "DESCRIPTION", "") or ""

    p: dict[str, Any] = {"api_name": api_name, "effect": effect}
    if roles:
        p["roles"] = roles
    if operations:
        p["operations"] = operations
    if resource_kind:
        p["resource_kind"] = resource_kind
    if resource:
        p["resource"] = resource
    if desc:
        p["description"] = desc
    cond = getattr(target, "CONDITION", None) if isinstance(target, type) else None
    if cond:
        p["condition"] = cond
    app._policies.append(p)
    return target


def _trait_def(app: "App", cls: Any, *, display: str = "") -> Any:
    if not dataclasses.is_dataclass(cls):
        cls = dataclasses.dataclass(cls)

    hints = typing.get_type_hints(cls)
    props = []
    for f in dataclasses.fields(cls):
        dt = _type_map(hints.get(f.name, str))
        props.append({"api_name": f.name, "data_type": dt})

    app._traits.append({
        "api_name": cls.__name__.lower(),
        "display": display or cls.__name__,
        "properties": props,
    })
    return cls


def _pipeline(app: "App", cls: Any, *, schedule: str = "",
              mode: str = "incremental") -> Any:
    steps = []
    for step in getattr(cls, "STEPS", []):
        s: dict[str, Any] = {
            "api_name": step["api_name"],
            "source": step["source"],
            "target": step["target"],
        }
        for key in ("expression", "language", "when", "on_error", "dynamic_source", "kind"):
            if step.get(key):
                s[key] = step[key]
        steps.append(s)

    p: dict[str, Any] = {
        "api_name": cls.__name__.lower(),
        "display": cls.__name__,
        "steps": steps,
        "mode": mode,
    }
    if schedule:
        p["schedule"] = {"Cron": schedule} if schedule != "manual" else "manual"
    ctx = getattr(cls, "CONTEXT", None)
    if ctx:
        p["context"] = ctx
    app._pipelines.append(p)
    return cls


class App:
    """Root workspace. One app = one ontology."""

    def __init__(self, name: str):
        self._name = name
        self._entities: list[dict] = []
        self._links: list[dict] = []
        self._actions: list[dict] = []
        self._agents: list[dict] = []
        self._policies: list[dict] = []
        self._traits: list[dict] = []
        self._pipelines: list[dict] = []
        self._ds = {"api_name": "memory", "adapter_type": "memory"}

    def compile(self) -> dict:
        spec: dict[str, Any] = {
            "version": "tesela.spec.v1",
            "workspace": {"api_name": self._name},
            "datasources": [self._ds],
        }
        if self._entities:
            spec["object_types"] = self._entities
        if self._links:
            spec["link_types"] = self._links
        if self._actions:
            spec["actions"] = self._actions
        if self._agents:
            spec["agents"] = self._agents
        if self._policies:
            spec["policies"] = self._policies
        if self._traits:
            spec["traits"] = self._traits
        if self._pipelines:
            spec["pipelines"] = self._pipelines
        return spec

    def compile_json(self, indent: int = 2) -> str:
        return json.dumps(self.compile(), indent=indent)

    def run(self):
        """Load native runtime."""
        from tesela.runtime import NativeRuntime
        return NativeRuntime.from_app(self)

    # ------------------------------------------------------------------
    # Decorator factories (support both @app.entity and @app.entity(...))
    # ------------------------------------------------------------------

    def entity(self, cls=None, *, datasource: str = "memory",
               resource: str = "", primary_key: str = "id"):
        """Declare an entity type."""
        def decorator(_cls):
            return _entity(self, _cls, datasource=datasource, resource=resource, primary_key=primary_key)
        if cls is not None and isinstance(cls, type):
            return decorator(cls)
        return decorator

    def link(self, cls=None, *, from_type: str = "", to_type: str = "",
             cardinality: str = "one_to_many"):
        """Declare a link between entities."""
        def decorator(_cls):
            return _link(self, _cls, from_type=from_type, to_type=to_type, cardinality=cardinality)
        if cls is not None and isinstance(cls, type):
            return decorator(cls)
        return decorator

    def action(self, fn=None, *, description: str = "", risk_level: str = "low",
               mode: str = "", handler_kind: str = "callback"):
        """Declare an action."""
        def decorator(_fn):
            return _action(self, _fn, description=description, risk_level=risk_level,
                           mode=mode, handler_kind=handler_kind)
        if fn is not None and callable(fn):
            return decorator(fn)
        return decorator

    def agent(self, cls=None, *, model: str = "", allowed_tools: list[str] | None = None):
        """Declare an agent."""
        def decorator(_cls):
            return _agent(self, _cls, model=model, allowed_tools=allowed_tools)
        if cls is not None and isinstance(cls, type):
            return decorator(cls)
        return decorator

    def policy(self, target=None, *, effect: str = "allow", roles: list[str] | None = None,
               operations: list[str] | None = None, resource_kind: str = "", resource: str = ""):
        """Declare a policy."""
        def decorator(_target):
            return _policy(self, _target, effect=effect, roles=roles, operations=operations,
                           resource_kind=resource_kind, resource=resource)
        if target is not None:
            return decorator(target)
        return decorator

    def trait_def(self, cls=None, *, display: str = ""):
        """Declare a trait (property mixin)."""
        def decorator(_cls):
            return _trait_def(self, _cls, display=display)
        if cls is not None and isinstance(cls, type):
            return decorator(cls)
        return decorator

    def pipeline(self, cls=None, *, schedule: str = "", mode: str = "incremental"):
        """Declare a pipeline."""
        def decorator(_cls):
            return _pipeline(self, _cls, schedule=schedule, mode=mode)
        if cls is not None and isinstance(cls, type):
            return decorator(cls)
        return decorator


# ------------------------------------------------------------------
# Standalone decorators (require explicit App reference)
# ------------------------------------------------------------------

def entity(app: App, *, datasource: str = "memory", resource: str = "", primary_key: str = "id"):
    """Standalone entity decorator: ``@entity(app, datasource='memory')``."""
    def decorator(cls):
        return _entity(app, cls, datasource=datasource, resource=resource, primary_key=primary_key)
    return decorator


def link(app: App, *, from_type: str = "", to_type: str = "", cardinality: str = "one_to_many"):
    """Standalone link decorator: ``@link(app, from_type='user')``."""
    def decorator(cls):
        return _link(app, cls, from_type=from_type, to_type=to_type, cardinality=cardinality)
    return decorator


def action(app: App, *, description: str = "", risk_level: str = "low",
           mode: str = "", handler_kind: str = "callback"):
    """Standalone action decorator: ``@action(app, risk_level='high')``."""
    def decorator(fn):
        return _action(app, fn, description=description, risk_level=risk_level,
                       mode=mode, handler_kind=handler_kind)
    return decorator


def agent(app: App, *, model: str = "", allowed_tools: list[str] | None = None):
    """Standalone agent decorator: ``@agent(app, model='gpt-4')``."""
    def decorator(cls):
        return _agent(app, cls, model=model, allowed_tools=allowed_tools)
    return decorator


def policy(app: App, *, effect: str = "allow", roles: list[str] | None = None,
           operations: list[str] | None = None, resource_kind: str = "", resource: str = ""):
    """Standalone policy decorator."""
    def decorator(target):
        return _policy(app, target, effect=effect, roles=roles, operations=operations,
                       resource_kind=resource_kind, resource=resource)
    return decorator


def trait_def(app: App, *, display: str = ""):
    """Standalone trait decorator."""
    def decorator(cls):
        return _trait_def(app, cls, display=display)
    return decorator


def pipeline(app: App, *, schedule: str = "", mode: str = "incremental"):
    """Standalone pipeline decorator."""
    def decorator(cls):
        return _pipeline(app, cls, schedule=schedule, mode=mode)
    return decorator

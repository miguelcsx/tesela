"""Declarative API — minimal, pythonic, readable.

Decorator-first ontology definitions. Use as ``@app.entity(datasource="memory")``
or standalone ``@entity(app, ...)``.
"""

from __future__ import annotations

import dataclasses
import inspect
import typing
import json
from pathlib import Path
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


class App:
    """Root workspace. One app = one ontology."""

    def __init__(self, name: str):
        self._name = name
        self._entities: list[dict] = []
        self._links: list[dict] = []
        self._actions: list[dict] = []
        self._agents: list[dict] = []
        self._ds = {"api_name": "memory", "adapter_type": "memory"}

    def compile(self) -> dict:
        return {
            "version": "tesela.spec.v1",
            "workspace": {"api_name": self._name},
            "datasources": [self._ds],
            "object_types": self._entities,
            "link_types": self._links,
            "actions": self._actions,
            "agents": self._agents,
        }

    def compile_json(self, indent: int = 2) -> str:
        return json.dumps(self.compile(), indent=indent)

    def run(self, lib_path: str | None = None):
        """Load native runtime."""
        from tesela.runtime import NativeRuntime
        path = Path(lib_path) if lib_path else Path(__file__).parent.parent.parent / "dist" / "libtesela.so"
        return NativeRuntime.from_app(self, library_path=path)

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

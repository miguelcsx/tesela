"""Dataclass + decorator API for defining Tesela ontologies with less boilerplate.

Usage::

    from dataclasses import dataclass, field
    from typing import Optional
    from tesela import App, object_type, action, agent

    app = App("crm")

    @object_type(app, datasource="memory", primary_key="id")
    @dataclass
    class Customer:
        id: str
        name: str
        email: str = field(default="", metadata={"indexed": True})
        revenue: float = 0.0
        notes: Optional[str] = None

    @action(app, risk="low")
    def create_customer(name: str, email: str) -> None:
        \"\"\"Create a new customer record.\"\"\"

    @agent(app, model="claude-sonnet-4-6")
    class SalesAgent:
        INSTRUCTIONS = "You help sales reps."
        ALLOWED_TOOLS = ["search_customer", "create_customer"]

    spec = app.compile()
"""

from __future__ import annotations

import dataclasses
import inspect
import typing
import uuid as _uuid
from typing import Any

from tesela.builder import App

_PY_TO_TESELA: dict[Any, str] = {
    str: "string",
    int: "integer",
    float: "float",
    bool: "boolean",
    _uuid.UUID: "uuid",
    list: "array",
}


def _map_type(py_type: Any) -> tuple[str, bool]:
    """Return (tesela_data_type, nullable) for a Python type hint.

    Handles both ``typing.Optional[T]`` (Python 3.8+) and ``T | None``
    (Python 3.10+) via ``typing.get_args``, which normalises both forms.
    """
    args = typing.get_args(py_type)
    if args and type(None) in args:
        inner = [a for a in args if a is not type(None)]
        py_type = inner[0] if len(inner) == 1 else str
        return _PY_TO_TESELA.get(py_type, "string"), True
    return _PY_TO_TESELA.get(py_type, "string"), False


def object_type(
    app: App,
    *,
    datasource: str = "memory",
    primary_key: str = "id",
    display: str = "",
) -> Any:
    """Decorator factory that registers a dataclass as a Tesela ObjectType.

    The decorated class is passed through unchanged so it remains a normal
    Python dataclass. Apply *after* ``@dataclass`` (i.e. this decorator is
    outermost).

    Example::

        @object_type(app, datasource="pg", primary_key="id")
        @dataclass
        class User:
            id: str
            name: str
            email: str = field(default="", metadata={"indexed": True})
    """

    def decorator(cls: Any) -> Any:
        if not dataclasses.is_dataclass(cls):
            cls = dataclasses.dataclass(cls)

        api_name = cls.__name__.lower()
        b = app.object_type(api_name)
        b.display(display or cls.__name__)
        b.source(datasource).primary_key(primary_key)

        hints = typing.get_type_hints(cls)
        for f in dataclasses.fields(cls):
            dt, nullable = _map_type(hints.get(f.name, str))
            meta = f.metadata
            b.property(
                f.name,
                dt,
                nullable=nullable,
                indexed=bool(meta.get("indexed")),
                unique=bool(meta.get("unique")),
                description=str(meta.get("description", "")),
            )

        b.done()
        return cls

    return decorator


def action(
    app: App,
    *,
    risk_level: str = "low",
    risk: str = "",
    handler: str = "callback",
    handler_target: str = "",
    display: str = "",
) -> Any:
    """Decorator factory that registers a function as a Tesela ActionType.

    The function signature is inspected to build the JSON input schema.
    The function itself is returned unchanged.

    Example::

        @action(app, risk_level="medium")
        def send_invoice(customer_id: str, amount: float) -> None:
            \"\"\"Send an invoice to a customer.\"\"\"
    """

    def decorator(fn: Any) -> Any:
        sig = inspect.signature(fn)
        hints = typing.get_type_hints(fn)
        properties: dict[str, Any] = {}
        for name, param in sig.parameters.items():
            py_type = hints.get(name, str)
            dt, _ = _map_type(py_type)
            properties[name] = {"type": dt}

        target = handler_target or fn.__name__
        (
            app.action(fn.__name__)
            .handler(handler, target)
            .description(inspect.getdoc(fn) or "")
            .risk_level(risk or risk_level)
            .input_schema({"type": "object", "properties": properties})
            .done()
        )
        return fn

    return decorator


def agent(
    app: App,
    *,
    model: str,
    display: str = "",
    apxm_skill_id: str = "",
) -> Any:
    """Decorator factory that registers a class as a Tesela Agent.

    The class may define class-level constants ``INSTRUCTIONS`` (str) and
    ``ALLOWED_TOOLS`` (sequence of str).  The class itself is returned
    unchanged.

    Example::

        @agent(app, model="claude-sonnet-4-6")
        class SalesAgent:
            INSTRUCTIONS = "You help sales reps with customer data."
            ALLOWED_TOOLS = ["search_customer", "create_customer"]
    """

    def decorator(cls: Any) -> Any:
        api_name = cls.__name__.lower()
        b = app.agent(api_name)
        b.model(model)
        b.display(display or cls.__name__)

        instructions = getattr(cls, "INSTRUCTIONS", None)
        if instructions:
            b.instructions(str(instructions))

        allowed_tools = getattr(cls, "ALLOWED_TOOLS", None)
        if allowed_tools:
            b.allow_tools(*allowed_tools)

        if apxm_skill_id:
            b.apxm_skill(apxm_skill_id)

        b.done()
        return cls

    return decorator


def link(
    app: App,
    *,
    from_type: str,
    to_type: str,
    cardinality: str = "one_to_many",
    display: str = "",
) -> Any:
    """Decorator factory that registers a class as a Tesela LinkType.

    The class may define ``MAPPINGS`` as a list of ``(from_prop, to_prop)``
    tuples.

    Example::

        @link(app, from_type="customer", to_type="order")
        class CustomerOrders:
            MAPPINGS = [("id", "customer_id")]
    """

    def decorator(cls: Any) -> Any:
        api_name = cls.__name__.lower()
        b = app.link(api_name, from_type, to_type, cardinality)
        if display:
            b.display(display)

        for from_prop, to_prop in getattr(cls, "MAPPINGS", []):
            b.mapping(from_prop, to_prop)

        b.done()
        return cls

    return decorator


def policy(
    app: App,
    *,
    effect: str = "allow",
    roles: list[str] | None = None,
    operations: list[str] | None = None,
    resource_kind: str = "",
    resource: str = "",
) -> Any:
    """Decorator factory that registers a function or class as a Tesela PolicyRule.

    For a function, the name becomes the api_name and the docstring the
    description. For a class, inspect ``EFFECT``, ``ROLES``, ``OPERATIONS``,
    ``RESOURCE_KIND``, ``RESOURCE``, ``CONDITION`` class attributes.

    Example::

        @policy(app, effect="allow", roles=["admin"], operations=["read", "write"])
        def admin_full_access():
            \"\"\"Admins have full access.\"\"\"
    """

    def decorator(target: Any) -> Any:
        if callable(target) and not isinstance(target, type):
            api_name = target.__name__
            b = app.policy(api_name)
            b.effect(effect)
            if roles:
                b.roles(*roles)
            if operations:
                b.operations(*operations)
            if resource_kind:
                b.resource(resource_kind, resource)
            b.description(inspect.getdoc(target) or "")
            b.done()
        else:
            api_name = target.__name__.lower()
            b = app.policy(api_name)
            b.effect(getattr(target, "EFFECT", effect))
            r = getattr(target, "ROLES", roles)
            if r:
                b.roles(*r)
            ops = getattr(target, "OPERATIONS", operations)
            if ops:
                b.operations(*ops)
            rk = getattr(target, "RESOURCE_KIND", resource_kind)
            rn = getattr(target, "RESOURCE", resource)
            if rk:
                b.resource(rk, rn)
            cond = getattr(target, "CONDITION", None)
            if cond:
                b.condition(cond)
            b.description(getattr(target, "DESCRIPTION", "") or "")
            b.done()
        return target

    return decorator


def trait_def(
    app: App,
    *,
    display: str = "",
) -> Any:
    """Decorator factory that registers a dataclass as a Tesela Trait (property mixin).

    Works like ``@object_type`` but produces a ``Trait`` instead of a full
    ``ObjectType``.

    Example::

        @trait_def(app)
        @dataclass
        class Auditable:
            created_at: str
            updated_at: str
            created_by: str
    """

    def decorator(cls: Any) -> Any:
        if not dataclasses.is_dataclass(cls):
            cls = dataclasses.dataclass(cls)

        api_name = cls.__name__.lower()
        b = app.trait(api_name)
        if display:
            b.display(display)

        hints = typing.get_type_hints(cls)
        for f in dataclasses.fields(cls):
            dt, nullable = _map_type(hints.get(f.name, str))
            b.property(f.name, dt, nullable=nullable)

        b.done()
        return cls

    return decorator


def pipeline(
    app: App,
    *,
    schedule: str = "",
    mode: str = "incremental",
    display: str = "",
) -> Any:
    """Decorator factory that registers a class as a Tesela TransformPipeline.

    The class should define ``STEPS`` as a list of dicts, each with at least
    ``api_name``, ``source``, ``target``.  Optional keys: ``expression``,
    ``language``, ``when``, ``on_error``, ``dynamic_source``, ``kind``.

    Example::

        @pipeline(app, schedule="0 * * * *")
        class DailySyncPipeline:
            STEPS = [
                {"api_name": "ingest", "source": "raw_data", "target": "clean_data"},
                {"api_name": "enrich", "source": "clean_data", "target": "enriched_data",
                 "when": "env == 'production'"},
            ]
    """

    def decorator(cls: Any) -> Any:
        api_name = cls.__name__.lower()
        b = app.pipeline(api_name)
        if display:
            b.display(display)
        if schedule:
            b.schedule(schedule)
        b.mode(mode)

        for step in getattr(cls, "STEPS", []):
            b.step(
                step["api_name"],
                step["source"],
                step["target"],
                expression=step.get("expression", ""),
                language=step.get("language", ""),
                when=step.get("when", ""),
                on_error=step.get("on_error", ""),
                dynamic_source=step.get("dynamic_source", ""),
                kind=step.get("kind", ""),
            )

        for key, value in getattr(cls, "CONTEXT", {}).items():
            b.context(key, value)

        b.done()
        return cls

    return decorator

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
            .risk_level(risk_level)
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

        b.done()
        return cls

    return decorator

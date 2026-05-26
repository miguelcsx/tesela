"""Declarative field helpers for Tesela ontology definitions.

Usage::

    from dataclasses import dataclass
    from tesela import App, entity, field

    app = App("crm")

    @app.entity(datasource="memory")
    @dataclass
    class Customer:
        id: str
        email: str = field(default="", indexed=True)
        revenue: float = 0.0
        notes: str | None = field(default=None, nullable=True)
"""

from __future__ import annotations

import dataclasses
from typing import Any


def field(
    default: Any = dataclasses.MISSING,
    *,
    default_factory: Any = dataclasses.MISSING,
    indexed: bool = False,
    unique: bool = False,
    nullable: bool = False,
    description: str = "",
    source_column: str = "",
    encrypted: bool = False,
    repr: bool = True,
    compare: bool = True,
    hash: bool | None = None,
) -> dataclasses.Field:
    """Return a ``dataclasses.Field`` with Tesela metadata.

    This is a thin wrapper around ``dataclasses.field`` that adds
    ``tesela.*`` keys to the field ``metadata`` dict so that the
    ``@entity`` / ``@object_type`` decorators can pick them up.

    Parameters
    ----------
    default
        Default value for the field (mutually exclusive with
        ``default_factory``).
    default_factory
        Zero-argument callable that produces the default value.
    indexed
        Whether the property should be indexed.
    unique
        Whether the property should be unique.
    nullable
        Whether NULL is allowed.
    description
        Human-readable description of the property.
    source_column
        Physical column name mapping.
    encrypted
        Whether the field is encrypted at rest.
    repr, compare, hash
        Passed through to ``dataclasses.field``.
    """
    meta: dict[str, Any] = {}
    if indexed:
        meta["indexed"] = True
    if unique:
        meta["unique"] = True
    if nullable:
        meta["nullable"] = True
    if description:
        meta["description"] = description
    if source_column:
        meta["source_column"] = source_column
    if encrypted:
        meta["encrypted"] = True

    kwargs: dict[str, Any] = {"repr": repr, "compare": compare, "metadata": meta}
    if hash is not None:
        kwargs["hash"] = hash
    if default is not dataclasses.MISSING:
        kwargs["default"] = default
    if default_factory is not dataclasses.MISSING:
        kwargs["default_factory"] = default_factory

    return dataclasses.field(**kwargs)

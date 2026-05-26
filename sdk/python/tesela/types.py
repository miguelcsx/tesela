"""Data types and runtime result models matching the canonical IR spec."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

String = "string"
Integer = "integer"
BigInt = "bigint"
Float = "float"
Decimal = "decimal"
Boolean = "boolean"
Date = "date"
Timestamp = "timestamp"
TimestampTZ = "timestamptz"
UUID = "uuid"
JSON = "json"
Geometry = "geometry"
Array = "array"
Enum = "enum"

SPEC_VERSION = "tesela.spec.v1"


@dataclass(slots=True)
class Record:
    primary_key: Any = None
    values: dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Record":
        return cls(primary_key=data.get("primary_key"), values=data.get("values", {}))


@dataclass(slots=True)
class Page:
    records: list[Record] = field(default_factory=list)
    next_cursor: str | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Page":
        return cls(
            records=[Record.from_dict(item) for item in data.get("records", [])],
            next_cursor=data.get("next_cursor"),
        )


@dataclass(slots=True)
class MutationResult:
    record: Record | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "MutationResult":
        record = data.get("record")
        return cls(record=Record.from_dict(record) if isinstance(record, dict) else None)


@dataclass(slots=True)
class ActionResult:
    status: str
    output: Any = None
    error: str | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ActionResult":
        return cls(status=data.get("status", ""), output=data.get("output"), error=data.get("error"))


@dataclass(slots=True)
class AggregateResult:
    groups: list[dict[str, Any]] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AggregateResult":
        return cls(groups=data.get("groups", []))


@dataclass(slots=True)
class UploadResult:
    run_id: str | None = None
    load_id: str | None = None
    rows_loaded: int = 0
    rows_skipped: int = 0
    skipped_rows: list[dict[str, Any]] = field(default_factory=list)
    quality: list[dict[str, Any]] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "UploadResult":
        return cls(
            run_id=data.get("run_id"),
            load_id=data.get("load_id"),
            rows_loaded=int(data.get("rows_loaded", 0)),
            rows_skipped=int(data.get("rows_skipped", 0)),
            skipped_rows=data.get("skipped_rows", []),
            quality=data.get("quality", []),
        )


@dataclass(slots=True)
class AgentRun:
    id: str
    status: str
    output: str | None = None
    error: str | None = None
    eval_passed: bool | None = None
    eval_score: float | None = None
    eval_notes: str | None = None

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "AgentRun":
        return cls(
            id=data.get("id", ""),
            status=data.get("status", ""),
            output=data.get("output"),
            error=data.get("error"),
            eval_passed=data.get("eval_passed"),
            eval_score=data.get("eval_score"),
            eval_notes=data.get("eval_notes"),
        )


@dataclass(slots=True)
class ExplainPlan:
    steps: list[dict[str, Any]] = field(default_factory=list)

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "ExplainPlan":
        return cls(steps=data.get("steps", []))


@dataclass(slots=True)
class HealthStatus:
    status: str
    spec_version: str
    workspace: str

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "HealthStatus":
        return cls(status=data.get("status", ""), spec_version=data.get("spec_version", ""), workspace=data.get("workspace", ""))


@dataclass(slots=True)
class Capabilities:
    values: dict[str, Any]

    @classmethod
    def from_dict(cls, data: dict[str, Any]) -> "Capabilities":
        return cls(values=data)

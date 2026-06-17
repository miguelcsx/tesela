from __future__ import annotations

import dataclasses
from typing import Any


class NativeError(RuntimeError):
    pass


@dataclasses.dataclass(frozen=True, slots=True)
class Record:
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
    record: Record | None = None
    rows_affected: int | None = None


@dataclasses.dataclass(frozen=True, slots=True)
class ActionResult:
    value: Any = None

    def to_dict(self) -> dict[str, Any]:
        if isinstance(self.value, dict):
            return self.value
        return {"value": self.value}


@dataclasses.dataclass(frozen=True, slots=True)
class ExplainPlan:
    steps: list[dict[str, Any]]


@dataclasses.dataclass(frozen=True, slots=True)
class HealthStatus:
    _data: dict[str, Any]

    def __getitem__(self, key: str) -> Any:
        return self._data[key]

    def get(self, key: str, default: Any = None) -> Any:
        return self._data.get(key, default)

    def to_dict(self) -> dict[str, Any]:
        return self._data


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

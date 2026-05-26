from __future__ import annotations

from typing import Any


def _replace_by(items: list[dict[str, Any]], item: dict[str, Any], key: str = "api_name") -> list[dict[str, Any]]:
    return [x for x in items if x.get(key) != item.get(key)] + [item]

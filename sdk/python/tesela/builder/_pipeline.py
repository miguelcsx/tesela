from __future__ import annotations

from typing import Any

from tesela.builder._common import _replace_by

class PipelineBuilder:
    def __init__(self, app: App, api_name: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "steps": []}

    def step(self, api_name: str, source: str, target: str, *,
             expression: str = "", language: str = "",
             when: str = "", on_error: str | dict = "",
             dynamic_source: str = "", kind: str = "") -> "PipelineBuilder":
        s: dict[str, Any] = {"api_name": api_name, "source": source, "target": target}
        if expression:
            s["expression"] = expression
        if language:
            s["language"] = language
        if when:
            s["when"] = when
        if on_error:
            s["on_error"] = on_error if isinstance(on_error, dict) else {"strategy": on_error}
        if dynamic_source:
            s["dynamic_source"] = dynamic_source
        if kind:
            s["kind"] = kind
        self._data["steps"].append(s)
        return self

    def decision(self, api_name: str, expression: str) -> "PipelineBuilder":
        return self.step(api_name, "_none", "_none", expression=expression, kind="decision")

    def schedule(self, cron: str) -> "PipelineBuilder":
        self._data["schedule"] = {"Cron": cron} if cron != "manual" else "manual"
        return self

    def mode(self, m: str) -> "PipelineBuilder":
        self._data["mode"] = m
        return self

    def display(self, v: str) -> "PipelineBuilder":
        self._data["display"] = v
        return self

    def description(self, v: str) -> "PipelineBuilder":
        self._data["description"] = v
        return self

    def context(self, key: str, value: Any) -> "PipelineBuilder":
        self._data.setdefault("context", {})[key] = value
        return self

    def metadata(self, key: str, value: Any) -> "PipelineBuilder":
        self._data.setdefault("metadata", {})[key] = value
        return self

    def done(self) -> App:
        self._app._pipelines = _replace_by(self._app._pipelines, self._data)
        return self._app

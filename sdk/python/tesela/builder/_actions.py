from __future__ import annotations

from typing import Any

from tesela.builder._common import _replace_by

class ActionBuilder:
    def __init__(self, app: App, api_name: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name}

    def subject(self, object_type: str) -> "ActionBuilder":
        self._data["subject"] = object_type
        return self

    def handler(self, kind: str, target: str = "", *, config: dict[str, Any] | None = None) -> "ActionBuilder":
        h: dict[str, Any] = {"kind": kind}
        if target:
            h["target"] = target
        if config:
            h["config"] = config
        self._data["handler"] = h
        return self

    def mode(self, m: str) -> "ActionBuilder":
        self._data["mode"] = m
        return self

    def risk_level(self, r: str) -> "ActionBuilder":
        self._data["risk_level"] = r
        return self

    def risk(self, r: str) -> "ActionBuilder":
        return self.risk_level(r)

    def input_schema(self, schema: dict[str, Any]) -> "ActionBuilder":
        self._data["input_schema"] = schema
        return self

    def output_schema(self, schema: dict[str, Any]) -> "ActionBuilder":
        self._data["output_schema"] = schema
        return self

    def description(self, v: str) -> "ActionBuilder":
        self._data["description"] = v
        return self

    def idempotency_key(self, k: str) -> "ActionBuilder":
        self._data["idempotency_key"] = k
        return self

    def metadata(self, key: str, value: Any) -> "ActionBuilder":
        self._data.setdefault("metadata", {})[key] = value
        return self

    def deprecated_at(self, v: str) -> "ActionBuilder":
        self._data["deprecated_at"] = v
        return self

    def done(self) -> App:
        self._app._actions = _replace_by(self._app._actions, self._data)
        return self._app
class PolicyBuilder:
    def __init__(self, app: App, api_name: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name}

    def effect(self, e: str) -> "PolicyBuilder":
        self._data["effect"] = e
        return self

    def roles(self, *r: str) -> "PolicyBuilder":
        self._data["roles"] = list(r)
        return self

    def operations(self, *ops: str) -> "PolicyBuilder":
        self._data["operations"] = list(ops)
        return self

    def resource(self, kind: str, name: str = "") -> "PolicyBuilder":
        self._data["resource_kind"] = kind
        if name:
            self._data["resource"] = name
        return self

    def row_filter(self, op: str, field: str, value: Any) -> "PolicyBuilder":
        self._data["row_filter"] = {"op": op, "field": field, "value": value}
        return self

    def redactions(self, *fields: str) -> "PolicyBuilder":
        self._data["redactions"] = list(fields)
        return self

    def condition(self, expr: str) -> "PolicyBuilder":
        self._data["condition"] = expr
        return self

    def description(self, v: str) -> "PolicyBuilder":
        self._data["description"] = v
        return self

    def priority(self, p: int) -> "PolicyBuilder":
        self._data["priority"] = p
        return self

    def done(self) -> App:
        self._app._policies = _replace_by(self._app._policies, self._data)
        return self._app
class AgentBuilder:
    def __init__(self, app: App, api_name: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name}

    def display(self, v: str) -> "AgentBuilder":
        self._data["display"] = v
        return self

    def model(self, model: str) -> "AgentBuilder":
        self._data["model"] = model
        return self

    def model_provider(self, provider: str) -> "AgentBuilder":
        self._data["model_provider"] = provider
        return self

    def instructions(self, text: str) -> "AgentBuilder":
        self._data["instructions"] = text
        return self

    def allow_tools(self, *tools: str) -> "AgentBuilder":
        self._data["allowed_tools"] = list(tools)
        return self

    def custom_tool(self, name: str) -> "AgentBuilder":
        self._data.setdefault("custom_tools", []).append(name)
        return self

    def requires_approval(self) -> "AgentBuilder":
        self._data["requires_approval"] = True
        return self

    def limits(self, *, max_tool_calls: int = 0, timeout_seconds: int = 0,
               max_tokens: int = 0) -> "AgentBuilder":
        lim: dict[str, Any] = {}
        if max_tool_calls:
            lim["max_tool_calls"] = max_tool_calls
        if timeout_seconds:
            lim["timeout_seconds"] = timeout_seconds
        if max_tokens:
            lim["max_tokens"] = max_tokens
        self._data["limits"] = lim
        return self

    def token_budget(self, budget: int) -> "AgentBuilder":
        self._data.setdefault("limits", {})["token_budget"] = budget
        return self

    def memory(self, *, enabled: bool = True, namespace: str = "", scope: str = "") -> "AgentBuilder":
        m: dict[str, Any] = {"enabled": enabled}
        if namespace:
            m["namespace"] = namespace
        if scope:
            m["scope"] = scope
        self._data["memory"] = m
        return self

    def context_source(self, name: str, kind: str, *, ref: str = "") -> "AgentBuilder":
        src: dict[str, Any] = {"name": name, "kind": kind}
        if ref:
            src["ref"] = ref
        self._data.setdefault("context_sources", []).append(src)
        return self

    def capability(self, c: str) -> "AgentBuilder":
        self._data.setdefault("capabilities", []).append(c)
        return self

    def output_schema(self, schema: dict[str, Any]) -> "AgentBuilder":
        self._data["output_schema"] = schema
        return self

    def output_object_type(self, ot: str) -> "AgentBuilder":
        self._data["output_object_type"] = ot
        return self

    def deprecated_at(self, v: str) -> "AgentBuilder":
        self._data["deprecated_at"] = v
        return self

    def metadata(self, key: str, value: Any) -> "AgentBuilder":
        self._data.setdefault("metadata", {})[key] = value
        return self

    def apxm_skill(self, skill_id: str) -> "AgentBuilder":
        self._data.setdefault("metadata", {})["apxm_skill_id"] = skill_id
        return self

    def done(self) -> App:
        self._app._agents = _replace_by(self._app._agents, self._data)
        return self._app
class CustomToolBuilder:
    def __init__(self, app: App, api_name: str, kind: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "kind": kind}

    def description(self, v: str) -> "CustomToolBuilder":
        self._data["description"] = v
        return self

    def handler(self, kind: str, target: str) -> "CustomToolBuilder":
        self._data["handler"] = {"kind": kind, "target": target}
        return self

    def input_schema(self, schema: dict[str, Any]) -> "CustomToolBuilder":
        self._data["input_schema"] = schema
        return self

    def deprecated_at(self, v: str) -> "CustomToolBuilder":
        self._data["deprecated_at"] = v
        return self

    def done(self) -> App:
        self._app._custom_tools = _replace_by(self._app._custom_tools, self._data)
        return self._app

"""Code-first builder that compiles to the Tesela canonical IR.

Every method call accumulates state; compile() produces the final spec dict,
compile_json() produces the JSON string ready for the runtime.
"""

from __future__ import annotations

import hashlib
import json
from typing import Any

from tesela.types import SPEC_VERSION


class App:
    """Root builder. One App = one workspace = one spec."""

    def __init__(self, workspace: str, display: str = ""):
        self._workspace = {"api_name": workspace}
        if display:
            self._workspace["display"] = display
        self._datasources: list[dict[str, Any]] = []
        self._traits: list[dict[str, Any]] = []
        self._object_types: list[dict[str, Any]] = []
        self._link_types: list[dict[str, Any]] = []
        self._actions: list[dict[str, Any]] = []
        self._roles: list[dict[str, Any]] = []
        self._policies: list[dict[str, Any]] = []
        self._agents: list[dict[str, Any]] = []
        self._custom_tools: list[dict[str, Any]] = []
        self._assets: list[dict[str, Any]] = []
        self._environments: list[dict[str, Any]] = []
        self._artifact_types: list[dict[str, Any]] = []
        self._upload_flows: list[dict[str, Any]] = []
        self._job_types: list[dict[str, Any]] = []
        self._event_types: list[dict[str, Any]] = []
        self._capability_grants: list[dict[str, Any]] = []
        self._aggregate_views: list[dict[str, Any]] = []

    def datasource(self, api_name: str, adapter_type: str, *, config: dict[str, Any] | None = None) -> "App":
        ds: dict[str, Any] = {"api_name": api_name, "adapter_type": adapter_type}
        if config:
            ds["config"] = config
        self._datasources = _replace_by(self._datasources, ds)
        return self

    def trait(self, api_name: str) -> TraitBuilder:
        return TraitBuilder(self, api_name)

    def object_type(self, api_name: str) -> ObjectTypeBuilder:
        return ObjectTypeBuilder(self, api_name)

    def link(self, api_name: str, from_type: str, to_type: str, cardinality: str = "one_to_many") -> LinkBuilder:
        return LinkBuilder(self, api_name, from_type, to_type, cardinality)

    def action(self, api_name: str) -> ActionBuilder:
        return ActionBuilder(self, api_name)

    def role(self, api_name: str, *, inherits: list[str] | None = None) -> "App":
        r: dict[str, Any] = {"api_name": api_name}
        if inherits:
            r["inherits"] = inherits
        self._roles = _replace_by(self._roles, r)
        return self

    def policy(self, api_name: str) -> PolicyBuilder:
        return PolicyBuilder(self, api_name)

    def agent(self, api_name: str) -> AgentBuilder:
        return AgentBuilder(self, api_name)

    def custom_tool(self, api_name: str, kind: str) -> CustomToolBuilder:
        return CustomToolBuilder(self, api_name, kind)

    def environment(self, api_name: str, *, display: str = "", config: dict[str, Any] | None = None) -> "App":
        env: dict[str, Any] = {"api_name": api_name}
        if display:
            env["display"] = display
        if config:
            env["config"] = config
        self._environments = _replace_by(self._environments, env)
        return self

    def asset(self, api_name: str) -> AssetBuilder:
        return AssetBuilder(self, api_name)

    def artifact(self, api_name: str, store: str, path_template: str) -> ArtifactBuilder:
        return ArtifactBuilder(self, api_name, store, path_template)

    def upload_flow(self, api_name: str, store: str, path_template: str) -> UploadFlowBuilder:
        return UploadFlowBuilder(self, api_name, store, path_template)

    def job(self, api_name: str, executor: str) -> JobBuilder:
        return JobBuilder(self, api_name, executor)

    def event(self, api_name: str, bus: str, topic: str) -> EventBuilder:
        return EventBuilder(self, api_name, bus, topic)

    def capability(self, api_name: str, resource_kind: str) -> CapabilityBuilder:
        return CapabilityBuilder(self, api_name, resource_kind)

    def aggregate_view(self, api_name: str, object_type: str) -> AggregateViewBuilder:
        return AggregateViewBuilder(self, api_name, object_type)

    def compile(self) -> dict[str, Any]:
        """Compile the builder state into the canonical IR dict."""
        spec: dict[str, Any] = {
            "version": SPEC_VERSION,
            "workspace": self._workspace,
        }
        if self._datasources:
            spec["datasources"] = self._datasources
        if self._traits:
            spec["traits"] = self._traits
        if self._object_types:
            spec["object_types"] = self._object_types
        if self._link_types:
            spec["link_types"] = self._link_types
        if self._actions:
            spec["actions"] = self._actions
        if self._roles:
            spec["roles"] = self._roles
        if self._policies:
            spec["policies"] = self._policies
        if self._agents:
            spec["agents"] = self._agents
        if self._custom_tools:
            spec["custom_tools"] = self._custom_tools
        if self._assets:
            spec["assets"] = self._assets
        if self._environments:
            spec["environments"] = self._environments
        if self._artifact_types:
            spec["artifact_types"] = self._artifact_types
        if self._upload_flows:
            spec["upload_flows"] = self._upload_flows
        if self._job_types:
            spec["job_types"] = self._job_types
        if self._event_types:
            spec["event_types"] = self._event_types
        if self._capability_grants:
            spec["capability_grants"] = self._capability_grants
        if self._aggregate_views:
            spec["aggregate_views"] = self._aggregate_views
        return spec

    def compile_json(self, indent: int = 2) -> str:
        """Compile and serialize to JSON."""
        return json.dumps(self.compile(), indent=indent)

    def hash(self) -> str:
        """SHA-256 of the deterministic JSON (sorted keys)."""
        raw = json.dumps(self.compile(), sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(raw.encode()).hexdigest()


class ObjectTypeBuilder:
    def __init__(self, app: App, api_name: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "properties": []}

    def source(self, datasource: str, resource: str = "") -> "ObjectTypeBuilder":
        src: dict[str, Any] = {"datasource": datasource}
        if resource:
            src["resource"] = resource
        self._data["source"] = src
        return self

    def property(self, api_name: str, data_type: str, *,
                 nullable: bool = False, indexed: bool = False, unique: bool = False,
                 tags: list[str] | None = None, description: str = "",
                 default: Any = None, markings: list[str] | None = None,
                 source_column: str = "", allowed_values: list[Any] | None = None,
                 sort_order: str = "", encrypted: bool = False) -> "ObjectTypeBuilder":
        p: dict[str, Any] = {"api_name": api_name, "data_type": data_type}
        if nullable:
            p["nullable"] = True
        if indexed:
            p["indexed"] = True
        if unique:
            p["unique"] = True
        if tags:
            p["tags"] = tags
        if description:
            p["description"] = description
        if default is not None:
            p["default"] = default
        if markings:
            p["markings"] = markings
        if source_column:
            p["source_column"] = source_column
        if allowed_values:
            p["allowed_values"] = allowed_values
        if sort_order:
            p["sort_order"] = sort_order
        if encrypted:
            p["encrypted"] = True
        self._data["properties"].append(p)
        return self

    def computed(self, api_name: str, data_type: str, *, language: str, expression: str) -> "ObjectTypeBuilder":
        p: dict[str, Any] = {
            "api_name": api_name,
            "data_type": data_type,
            "computed": {"language": language, "expression": expression},
        }
        self._data["properties"].append(p)
        return self

    def quality(self, api_name: str, kind: str, *, property: str = "", severity: str = "",
                args: dict[str, Any] | None = None) -> "ObjectTypeBuilder":
        q: dict[str, Any] = {"api_name": api_name, "kind": kind}
        if property:
            q["property"] = property
        if severity:
            q["severity"] = severity
        if args:
            q["args"] = args
        self._data.setdefault("quality_rules", []).append(q)
        return self

    def primary_key(self, field: str) -> "ObjectTypeBuilder":
        self._data["primary_key"] = field
        return self

    def use_trait(self, *trait_names: str) -> "ObjectTypeBuilder":
        self._data.setdefault("traits", []).extend(trait_names)
        return self

    def display(self, v: str) -> "ObjectTypeBuilder":
        self._data["display"] = v
        return self

    def description(self, v: str) -> "ObjectTypeBuilder":
        self._data["description"] = v
        return self

    def metadata(self, key: str, value: Any) -> "ObjectTypeBuilder":
        self._data.setdefault("metadata", {})[key] = value
        return self

    def tag(self, *tags: str) -> "ObjectTypeBuilder":
        self._data.setdefault("tags", []).extend(tags)
        return self

    def index(self, api_name: str, properties: list[str], *, unique: bool = False) -> "ObjectTypeBuilder":
        idx: dict[str, Any] = {"api_name": api_name, "properties": properties}
        if unique:
            idx["unique"] = True
        self._data.setdefault("indexes", []).append(idx)
        return self

    def temporal(self, *, valid_start: str = "", valid_end: str = "",
                 sys_start: str = "", sys_end: str = "") -> "ObjectTypeBuilder":
        t: dict[str, Any] = {}
        if valid_start:
            t["valid_time_start"] = valid_start
        if valid_end:
            t["valid_time_end"] = valid_end
        if sys_start:
            t["system_time_start"] = sys_start
        if sys_end:
            t["system_time_end"] = sys_end
        self._data["temporal"] = t
        return self

    def lifecycle(self, *, soft_delete: bool = False, archival: bool = False,
                  retention_days: int = 0) -> "ObjectTypeBuilder":
        lc: dict[str, Any] = {}
        if soft_delete:
            lc["soft_delete"] = True
        if archival:
            lc["archival"] = True
        if retention_days:
            lc["retention_days"] = retention_days
        self._data["lifecycle"] = lc
        return self

    def scoring(self, *, enabled: bool = False, model: str = "", threshold: float = 0.0) -> "ObjectTypeBuilder":
        s: dict[str, Any] = {}
        if enabled:
            s["enabled"] = True
        if model:
            s["model"] = model
        if threshold:
            s["threshold"] = threshold
        self._data["scoring"] = s
        return self

    def quality_rule(self, api_name: str, kind: str, *, property: str = "",
                     severity: str = "", args: dict[str, Any] | None = None) -> "ObjectTypeBuilder":
        return self.quality(api_name, kind, property=property, severity=severity, args=args)

    def lineage(self, source: str, target: str, relation: str = "derived_from") -> "ObjectTypeBuilder":
        self._data.setdefault("lineage", []).append({
            "source": source, "target": target, "relation": relation,
        })
        return self

    def classification(self, *, sensitivity: str = "", owner: str = "",
                       data_domain: str = "") -> "ObjectTypeBuilder":
        c: dict[str, Any] = {}
        if sensitivity:
            c["sensitivity"] = sensitivity
        if owner:
            c["owner"] = owner
        if data_domain:
            c["data_domain"] = data_domain
        self._data["classification"] = c
        return self

    def deprecated_at(self, v: str) -> "ObjectTypeBuilder":
        self._data["deprecated_at"] = v
        return self

    def done(self) -> App:
        self._app._object_types = _replace_by(self._app._object_types, self._data)
        return self._app


class TraitBuilder:
    def __init__(self, app: App, api_name: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "properties": []}

    def property(self, api_name: str, data_type: str, **kwargs: Any) -> "TraitBuilder":
        p: dict[str, Any] = {"api_name": api_name, "data_type": data_type}
        p.update({k: v for k, v in kwargs.items() if v})
        self._data["properties"].append(p)
        return self

    def display(self, v: str) -> "TraitBuilder":
        self._data["display"] = v
        return self

    def done(self) -> App:
        self._app._traits = _replace_by(self._app._traits, self._data)
        return self._app


class LinkBuilder:
    def __init__(self, app: App, api_name: str, from_type: str, to_type: str, cardinality: str):
        self._app = app
        self._data: dict[str, Any] = {
            "api_name": api_name,
            "from": from_type,
            "to": to_type,
            "cardinality": cardinality,
        }

    def mapping(self, from_prop: str, to_prop: str) -> "LinkBuilder":
        self._data.setdefault("mappings", []).append({
            "from_property": from_prop, "to_property": to_prop,
        })
        return self

    def junction(self, datasource: str, resource: str, from_col: str, to_col: str) -> "LinkBuilder":
        self._data["junction"] = {
            "datasource": datasource, "resource": resource,
            "from_column": from_col, "to_column": to_col,
        }
        return self

    def source(self, datasource: str, resource: str = "") -> "LinkBuilder":
        src: dict[str, Any] = {"datasource": datasource}
        if resource:
            src["resource"] = resource
        self._data["source"] = src
        return self

    def metadata(self, key: str, value: Any) -> "LinkBuilder":
        self._data.setdefault("metadata", {})[key] = value
        return self

    def deprecated_at(self, v: str) -> "LinkBuilder":
        self._data["deprecated_at"] = v
        return self

    def done(self) -> App:
        self._app._link_types = _replace_by(self._app._link_types, self._data)
        return self._app


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


class AssetBuilder:
    def __init__(self, app: App, api_name: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "properties": []}

    def sink(self, datasource: str, resource: str) -> "AssetBuilder":
        self._data["sink"] = {"datasource": datasource, "resource": resource}
        return self

    def property(self, api_name: str, data_type: str) -> "AssetBuilder":
        self._data["properties"].append({"api_name": api_name, "data_type": data_type})
        return self

    def tag(self, *tags: str) -> "AssetBuilder":
        self._data.setdefault("tags", []).extend(tags)
        return self

    def deprecated_at(self, v: str) -> "AssetBuilder":
        self._data["deprecated_at"] = v
        return self

    def done(self) -> App:
        self._app._assets = _replace_by(self._app._assets, self._data)
        return self._app


class ArtifactBuilder:
    def __init__(self, app: App, api_name: str, store: str, path_template: str):
        self._app = app
        self._data: dict[str, Any] = {
            "api_name": api_name,
            "store": store,
            "path_template": path_template,
        }

    def media_type(self, value: str) -> "ArtifactBuilder":
        self._data["media_type"] = value
        return self

    def property(self, api_name: str, data_type: str) -> "ArtifactBuilder":
        self._data.setdefault("metadata_schema", []).append({"api_name": api_name, "data_type": data_type})
        return self

    def state(self, *states: str) -> "ArtifactBuilder":
        self._data.setdefault("lifecycle", []).extend(states)
        return self

    def description(self, value: str) -> "ArtifactBuilder":
        self._data["description"] = value
        return self

    def metadata(self, key: str, value: Any) -> "ArtifactBuilder":
        self._data.setdefault("metadata", {})[key] = value
        return self

    def done(self) -> App:
        self._app._artifact_types = _replace_by(self._app._artifact_types, self._data)
        return self._app


class UploadFlowBuilder:
    def __init__(self, app: App, api_name: str, store: str, path_template: str):
        self._app = app
        self._data: dict[str, Any] = {
            "api_name": api_name,
            "store": store,
            "path_template": path_template,
        }

    def accepted_formats(self, *formats: str) -> "UploadFlowBuilder":
        self._data["accepted_formats"] = list(formats)
        return self

    def max_bytes(self, value: int) -> "UploadFlowBuilder":
        self._data["max_bytes"] = value
        return self

    def target(self, object_type: str) -> "UploadFlowBuilder":
        self._data["target_object_type"] = object_type
        return self

    def mapping(self, source_column: str, target_property: str, *, required: bool = False,
                type_coercion: str = "") -> "UploadFlowBuilder":
        mapping: dict[str, Any] = {"source_column": source_column, "target_property": target_property}
        if required:
            mapping["required"] = True
        if type_coercion:
            mapping["type_coercion"] = type_coercion
        self._data.setdefault("mappings", []).append(mapping)
        return self

    def quality_rule(self, api_name: str, kind: str, *, property: str = "",
                     severity: str = "", args: dict[str, Any] | None = None) -> "UploadFlowBuilder":
        rule: dict[str, Any] = {"api_name": api_name, "kind": kind}
        if property:
            rule["property"] = property
        if severity:
            rule["severity"] = severity
        if args:
            rule["args"] = args
        self._data.setdefault("quality_rules", []).append(rule)
        return self

    def discover_schema(self, enabled: bool = True) -> "UploadFlowBuilder":
        self._data["discover_schema"] = enabled
        return self

    def rollback_required(self, enabled: bool = True) -> "UploadFlowBuilder":
        self._data["rollback_required"] = enabled
        return self

    def done(self) -> App:
        self._app._upload_flows = _replace_by(self._app._upload_flows, self._data)
        return self._app


class JobBuilder:
    def __init__(self, app: App, api_name: str, executor: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "executor": executor}

    def states(self, *states: str) -> "JobBuilder":
        self._data["states"] = list(states)
        return self

    def idempotency_key(self, value: str) -> "JobBuilder":
        self._data["idempotency_key"] = value
        return self

    def start_event(self, event: str) -> "JobBuilder":
        self._data["start_event"] = event
        return self

    def result_event(self, event: str) -> "JobBuilder":
        self._data["result_event"] = event
        return self

    def input_schema(self, schema: dict[str, Any]) -> "JobBuilder":
        self._data["input_schema"] = schema
        return self

    def output_schema(self, schema: dict[str, Any]) -> "JobBuilder":
        self._data["output_schema"] = schema
        return self

    def done(self) -> App:
        self._app._job_types = _replace_by(self._app._job_types, self._data)
        return self._app


class EventBuilder:
    def __init__(self, app: App, api_name: str, bus: str, topic: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "bus": bus, "topic": topic}

    def payload_schema(self, schema: dict[str, Any]) -> "EventBuilder":
        self._data["payload_schema"] = schema
        return self

    def correlation_keys(self, *keys: str) -> "EventBuilder":
        self._data["correlation_keys"] = list(keys)
        return self

    def done(self) -> App:
        self._app._event_types = _replace_by(self._app._event_types, self._data)
        return self._app


class CapabilityBuilder:
    def __init__(self, app: App, api_name: str, resource_kind: str):
        self._app = app
        self._data: dict[str, Any] = {"api_name": api_name, "resource_kind": resource_kind}

    def resource(self, name: str) -> "CapabilityBuilder":
        self._data["resource"] = name
        return self

    def operations(self, *operations: str) -> "CapabilityBuilder":
        self._data["operations"] = list(operations)
        return self

    def ttl_seconds(self, value: int) -> "CapabilityBuilder":
        self._data["ttl_seconds"] = value
        return self

    def constraint(self, key: str, value: Any) -> "CapabilityBuilder":
        self._data.setdefault("constraints", {})[key] = value
        return self

    def done(self) -> App:
        self._app._capability_grants = _replace_by(self._app._capability_grants, self._data)
        return self._app


class AggregateViewBuilder:
    def __init__(self, app: App, api_name: str, object_type: str):
        self._app = app
        self._data: dict[str, Any] = {
            "api_name": api_name,
            "object_type": object_type,
            "require_pushdown": True,
        }

    def group_by(self, *properties: str) -> "AggregateViewBuilder":
        self._data["group_by"] = list(properties)
        return self

    def measure(self, function: str, alias: str, *, property: str = "", distinct: bool = False) -> "AggregateViewBuilder":
        measure: dict[str, Any] = {"function": function, "alias": alias}
        if property:
            measure["property"] = property
        if distinct:
            measure["distinct"] = True
        self._data.setdefault("measures", []).append(measure)
        return self

    def time_bucket(self, property: str, interval: str, *, timezone: str = "") -> "AggregateViewBuilder":
        bucket: dict[str, Any] = {"property": property, "interval": interval}
        if timezone:
            bucket["timezone"] = timezone
        self._data["time_bucket"] = bucket
        return self

    def spatial_extent(self, property: str, output: str = "bbox") -> "AggregateViewBuilder":
        self._data["spatial_extent"] = {"property": property, "output": output}
        return self

    def require_pushdown(self, enabled: bool = True) -> "AggregateViewBuilder":
        self._data["require_pushdown"] = enabled
        return self

    def done(self) -> App:
        self._app._aggregate_views = _replace_by(self._app._aggregate_views, self._data)
        return self._app


def _replace_by(items: list[dict[str, Any]], new: dict[str, Any]) -> list[dict[str, Any]]:
    """Replace an item by api_name or append."""
    name = new.get("api_name")
    for i, item in enumerate(items):
        if item.get("api_name") == name:
            items[i] = new
            return items
    items.append(new)
    return items

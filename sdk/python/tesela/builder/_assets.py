from __future__ import annotations

from typing import Any

from tesela.builder._common import _replace_by

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

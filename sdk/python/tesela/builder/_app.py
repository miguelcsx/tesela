from __future__ import annotations

import hashlib
import json
from typing import Any

from tesela.types import SPEC_VERSION
from tesela.builder._common import _replace_by
from tesela.builder._ontology import ObjectTypeBuilder, TraitBuilder, LinkBuilder
from tesela.builder._actions import ActionBuilder, PolicyBuilder, AgentBuilder, CustomToolBuilder
from tesela.builder._assets import AssetBuilder, ArtifactBuilder, UploadFlowBuilder, JobBuilder, EventBuilder, CapabilityBuilder, AggregateViewBuilder
from tesela.builder._pipeline import PipelineBuilder

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
        self._pipelines: list[dict[str, Any]] = []

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

    def pipeline(self, api_name: str) -> "PipelineBuilder":
        return PipelineBuilder(self, api_name)

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
        if self._pipelines:
            spec["pipelines"] = self._pipelines
        return spec

    def compile_json(self, indent: int = 2) -> str:
        """Compile and serialize to JSON."""
        return json.dumps(self.compile(), indent=indent)

    def hash(self) -> str:
        """SHA-256 of the deterministic JSON (sorted keys)."""
        raw = json.dumps(self.compile(), sort_keys=True, separators=(",", ":"))
        return hashlib.sha256(raw.encode()).hexdigest()

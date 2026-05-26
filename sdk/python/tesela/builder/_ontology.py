from __future__ import annotations

from typing import Any

from tesela.builder._common import _replace_by

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

    def display(self, v: str) -> "LinkBuilder":
        self._data["display"] = v
        return self

    def deprecated_at(self, v: str) -> "LinkBuilder":
        self._data["deprecated_at"] = v
        return self

    def done(self) -> App:
        self._app._link_types = _replace_by(self._app._link_types, self._data)
        return self._app

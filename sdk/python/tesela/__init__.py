"""Tesela — declarative ontology-driven applications.

Build ontologies with dataclasses, run with native Rust runtime.
"""

from tesela.types import String, Integer, Float, Boolean, Date, Timestamp, UUID, JSON
from tesela.builder import App
from tesela.decorators import object_type, action, agent, link, policy, trait_def, pipeline
from tesela.declarative import entity
from tesela.fields import field
from tesela.runtime import NativeRuntime, AsyncNativeRuntime, Record, Page, MutationResult, ActionResult

__all__ = [
    "App", "object_type", "entity", "link", "action", "agent", "field",
    "policy", "trait_def", "pipeline",
    "NativeRuntime", "AsyncNativeRuntime",
    "Record", "Page", "MutationResult", "ActionResult",
    "String", "Integer", "Float", "Boolean", "Date", "Timestamp", "UUID", "JSON",
]

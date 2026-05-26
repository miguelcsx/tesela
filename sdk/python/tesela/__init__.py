"""Tesela — declarative ontology-driven applications.

Build ontologies with dataclasses, run with native Rust runtime.
"""

from tesela.types import String, Integer, Float, Boolean, Date, Timestamp, UUID, JSON
from tesela.builder import App
from tesela.decorators import object_type, action, agent
from tesela.declarative import entity, link
from tesela.fields import field
from tesela.runtime import NativeRuntime, AsyncNativeRuntime, Record, Page, MutationResult, ActionResult

__all__ = [
    "App", "object_type", "entity", "link", "action", "agent", "field",
    "NativeRuntime", "AsyncNativeRuntime",
    "Record", "Page", "MutationResult", "ActionResult",
    "String", "Integer", "Float", "Boolean", "Date", "Timestamp", "UUID", "JSON",
]

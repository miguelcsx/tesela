"""Tests for the decorator / dataclass API."""

from __future__ import annotations

import uuid
from dataclasses import dataclass, field
from typing import Optional

from tesela import App, action, agent, object_type
from tesela.types import SPEC_VERSION


def test_object_type_decorator_basic():
    app = App("test")

    @object_type(app, datasource="memory", primary_key="id")
    @dataclass
    class Product:
        id: str
        name: str
        price: float

    spec = app.compile()
    assert spec["version"] == SPEC_VERSION
    ots = spec["object_types"]
    assert len(ots) == 1
    ot = ots[0]
    assert ot["api_name"] == "product"
    assert ot["primary_key"] == "id"
    props = {p["api_name"]: p for p in ot["properties"]}
    assert props["id"]["data_type"] == "string"
    assert props["price"]["data_type"] == "float"
    assert not props["id"].get("nullable")


def test_object_type_optional_fields():
    app = App("test")

    @object_type(app, datasource="memory", primary_key="id")
    @dataclass
    class Item:
        id: str
        note: Optional[str] = None

    spec = app.compile()
    props = {p["api_name"]: p for p in spec["object_types"][0]["properties"]}
    assert props["note"]["nullable"] is True
    assert props["note"]["data_type"] == "string"


def test_object_type_indexed_unique_via_metadata():
    app = App("test")

    @object_type(app, datasource="pg", primary_key="id")
    @dataclass
    class User:
        id: str
        email: str = field(default="", metadata={"indexed": True, "unique": True})

    spec = app.compile()
    props = {p["api_name"]: p for p in spec["object_types"][0]["properties"]}
    assert props["email"]["indexed"] is True
    assert props["email"]["unique"] is True
    assert spec["object_types"][0]["source"]["datasource"] == "pg"


def test_object_type_uuid_field():
    app = App("test")

    @object_type(app, datasource="memory", primary_key="id")
    @dataclass
    class Entity:
        id: uuid.UUID

    spec = app.compile()
    props = {p["api_name"]: p for p in spec["object_types"][0]["properties"]}
    assert props["id"]["data_type"] == "uuid"


def test_action_decorator():
    app = App("test")

    @action(app, risk="medium")
    def send_invoice(customer_id: str, amount: float) -> None:
        """Send an invoice to a customer."""

    spec = app.compile()
    assert len(spec["actions"]) == 1
    act = spec["actions"][0]
    assert act["api_name"] == "send_invoice"
    assert act["risk_level"] == "medium"
    assert act["description"] == "Send an invoice to a customer."
    props = act["input_schema"]["properties"]
    assert props["customer_id"]["type"] == "string"
    assert props["amount"]["type"] == "float"


def test_action_decorator_passthrough():
    app = App("test")

    @action(app, risk="low")
    def noop() -> None:
        pass

    assert callable(noop)


def test_agent_decorator():
    app = App("test")

    @agent(app, model="claude-sonnet-4-6")
    class Analyst:
        INSTRUCTIONS = "You are a data analyst."
        ALLOWED_TOOLS = ["search_order", "get_order"]

    spec = app.compile()
    assert len(spec["agents"]) == 1
    ag = spec["agents"][0]
    assert ag["api_name"] == "analyst"
    assert ag["model"] == "claude-sonnet-4-6"
    assert ag["instructions"] == "You are a data analyst."
    assert ag["allowed_tools"] == ["search_order", "get_order"]


def test_agent_class_passthrough():
    app = App("test")

    @agent(app, model="claude-haiku-4-5-20251001")
    class MyAgent:
        INSTRUCTIONS = "test"

    assert MyAgent.INSTRUCTIONS == "test"


def test_full_declarative_workflow():
    app = App("crm", display="CRM Workspace")

    @object_type(app, datasource="memory", primary_key="id")
    @dataclass
    class Customer:
        id: str
        name: str
        email: str = field(default="", metadata={"indexed": True})
        revenue: float = 0.0
        notes: Optional[str] = None

    @action(app, risk="low")
    def create_customer(name: str, email: str) -> None:
        """Create a new customer record."""

    @agent(app, model="claude-sonnet-4-6", apxm_skill_id="sales-skill")
    class SalesAgent:
        INSTRUCTIONS = "Help sales reps."
        ALLOWED_TOOLS = ["search_customer", "create_customer"]

    spec = app.compile()
    assert spec["workspace"]["api_name"] == "crm"
    assert len(spec["object_types"]) == 1
    assert len(spec["actions"]) == 1
    assert len(spec["agents"]) == 1
    assert spec["agents"][0]["metadata"]["apxm_skill_id"] == "sales-skill"

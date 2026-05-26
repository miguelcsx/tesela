"""Declarative ontology definition using dataclasses and decorators."""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

from tesela import App, action, agent, object_type

app = App("crm", display="CRM Workspace")


@object_type(app, datasource="memory", primary_key="id")
@dataclass
class Customer:
    id: str
    name: str
    email: str = field(default="", metadata={"indexed": True})
    revenue: float = 0.0
    notes: Optional[str] = None


@object_type(app, datasource="memory", primary_key="id")
@dataclass
class Order:
    id: str
    customer_id: str = field(default="", metadata={"indexed": True})
    amount: float = 0.0
    status: str = "pending"


@action(app, risk="low")
def create_customer(name: str, email: str) -> None:
    """Create a new customer record."""


@action(app, risk="medium")
def close_order(order_id: str, reason: str) -> None:
    """Close an order with a reason."""


@agent(app, model="claude-sonnet-4-6")
class SalesAgent:
    INSTRUCTIONS = "You help sales reps analyse customers and manage orders."
    ALLOWED_TOOLS = ["search_customer", "search_order", "create_customer", "close_order"]


if __name__ == "__main__":
    spec = app.compile()
    print(f"Workspace: {spec['workspace']['api_name']}")
    print(f"Object types: {[ot['api_name'] for ot in spec.get('object_types', [])]}")
    print(f"Actions: {[a['api_name'] for a in spec.get('actions', [])]}")
    print(f"Agents: {[ag['api_name'] for ag in spec.get('agents', [])]}")

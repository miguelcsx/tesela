"""Tests for the Tesela Python builder SDK."""

import json

from tesela import App, String, Integer, Boolean, Timestamp
from tesela.types import SPEC_VERSION


def test_basic_compile():
    app = App("acme")
    app.datasource("main", "postgres", config={"host": "localhost"})
    app.object_type("customer") \
        .source("main", "customers") \
        .property("id", String, indexed=True, unique=True) \
        .property("email", String, tags=["pii"]) \
        .property("age", Integer) \
        .primary_key("id") \
        .done()

    spec = app.compile()
    assert spec["version"] == SPEC_VERSION
    assert spec["workspace"]["api_name"] == "acme"
    assert len(spec["datasources"]) == 1
    assert spec["datasources"][0]["adapter_type"] == "postgres"
    assert len(spec["object_types"]) == 1
    ot = spec["object_types"][0]
    assert ot["api_name"] == "customer"
    assert ot["primary_key"] == "id"
    assert len(ot["properties"]) == 3
    assert ot["properties"][0]["indexed"] is True
    assert ot["properties"][1]["tags"] == ["pii"]


def test_compile_json_roundtrip():
    app = App("test")
    app.datasource("main", "memory")
    app.object_type("item") \
        .source("main", "items") \
        .property("id", String) \
        .primary_key("id") \
        .done()

    raw = app.compile_json()
    parsed = json.loads(raw)
    assert parsed["version"] == SPEC_VERSION
    assert parsed["object_types"][0]["api_name"] == "item"


def test_traits():
    app = App("test")
    app.datasource("main", "memory")
    app.trait("auditable") \
        .property("created_at", Timestamp) \
        .property("updated_at", Timestamp) \
        .done()
    app.object_type("order") \
        .source("main", "orders") \
        .use_trait("auditable") \
        .property("id", String) \
        .primary_key("id") \
        .done()

    spec = app.compile()
    assert len(spec["traits"]) == 1
    assert spec["traits"][0]["api_name"] == "auditable"
    assert spec["object_types"][0]["traits"] == ["auditable"]


def test_links():
    app = App("test")
    app.datasource("main", "memory")
    app.object_type("customer").source("main", "c").property("id", String).primary_key("id").done()
    app.object_type("order").source("main", "o").property("id", String).primary_key("id").done()
    app.link("customer_orders", "customer", "order", "one_to_many") \
        .mapping("id", "customer_id") \
        .done()

    spec = app.compile()
    assert len(spec["link_types"]) == 1
    lt = spec["link_types"][0]
    assert lt["from"] == "customer"
    assert lt["to"] == "order"
    assert lt["cardinality"] == "one_to_many"


def test_actions():
    app = App("test")
    app.datasource("main", "memory")
    app.action("cancel_order") \
        .subject("order") \
        .handler("webhook", "https://api.example.com/cancel") \
        .mode("sync") \
        .risk("high") \
        .done()

    spec = app.compile()
    assert len(spec["actions"]) == 1
    action = spec["actions"][0]
    assert action["handler"]["kind"] == "webhook"
    assert action["risk_level"] == "high"


def test_roles():
    app = App("test")
    app.role("admin")
    app.role("analyst", inherits=["admin"])

    spec = app.compile()
    assert len(spec["roles"]) == 2
    assert spec["roles"][1]["inherits"] == ["admin"]


def test_policies():
    app = App("test")
    app.policy("admin_full_access") \
        .effect("allow") \
        .roles("admin") \
        .operations("read", "mutate", "execute") \
        .done()
    app.policy("tenant_isolation") \
        .effect("allow") \
        .row_filter("eq", "tenant_id", "{{actor.tenant_id}}") \
        .done()

    spec = app.compile()
    assert len(spec["policies"]) == 2
    assert spec["policies"][0]["effect"] == "allow"
    assert spec["policies"][1]["row_filter"]["op"] == "eq"


def test_agents():
    app = App("test")
    app.agent("analyst") \
        .model("claude-sonnet-4-6") \
        .instructions("You are a data analyst.") \
        .allow_tools("search", "get") \
        .apxm_skill("skill-analyst") \
        .limits(max_tool_calls=20, timeout_seconds=120) \
        .memory(enabled=True, scope="user") \
        .requires_approval() \
        .done()

    spec = app.compile()
    assert len(spec["agents"]) == 1
    agent = spec["agents"][0]
    assert agent["model"] == "claude-sonnet-4-6"
    assert agent["requires_approval"] is True
    assert agent["limits"]["max_tool_calls"] == 20
    assert agent["memory"]["scope"] == "user"
    assert agent["metadata"]["apxm_skill_id"] == "skill-analyst"


def test_custom_tools():
    app = App("test")
    app.custom_tool("lookup_user", "webhook") \
        .description("Look up user by email") \
        .handler("webhook", "https://api.example.com/lookup") \
        .done()

    spec = app.compile()
    assert len(spec["custom_tools"]) == 1
    assert spec["custom_tools"][0]["kind"] == "webhook"


def test_assets():
    app = App("test")
    app.datasource("main", "memory")
    app.asset("customer_import") \
        .sink("main", "customers") \
        .property("name", String) \
        .tag("import", "etl") \
        .done()

    spec = app.compile()
    assert len(spec["assets"]) == 1
    assert spec["assets"][0]["sink"]["datasource"] == "main"
    assert spec["assets"][0]["tags"] == ["import", "etl"]


def test_hash_deterministic():
    app = App("test")
    app.datasource("main", "memory")
    app.object_type("item").source("main", "x").property("id", String).primary_key("id").done()

    h1 = app.hash()
    h2 = app.hash()
    assert h1 == h2
    assert len(h1) == 64


def test_hash_changes():
    app1 = App("test")
    app1.datasource("main", "memory")
    app1.object_type("a").source("main", "x").property("id", String).primary_key("id").done()

    app2 = App("test")
    app2.datasource("main", "memory")
    app2.object_type("b").source("main", "x").property("id", String).primary_key("id").done()

    assert app1.hash() != app2.hash()


def test_replace_by_api_name():
    app = App("test")
    app.datasource("main", "memory")
    app.datasource("main", "postgres")

    spec = app.compile()
    assert len(spec["datasources"]) == 1
    assert spec["datasources"][0]["adapter_type"] == "postgres"


def test_object_type_metadata_and_classification():
    app = App("test")
    app.datasource("main", "memory")
    app.object_type("item") \
        .source("main", "items") \
        .property("id", String) \
        .primary_key("id") \
        .metadata("team", "platform") \
        .classification(sensitivity="internal") \
        .done()

    spec = app.compile()
    assert spec["object_types"][0]["metadata"]["team"] == "platform"
    assert spec["object_types"][0]["classification"]["sensitivity"] == "internal"

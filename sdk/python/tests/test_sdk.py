from __future__ import annotations

from concurrent.futures import ThreadPoolExecutor

import pytest

from tesela import MemoryStore, Runtime, Spec, tool_definitions


ACTOR = {"user_id": "tester", "roles": ["admin"]}


def sample_spec():
    spec = Spec("demo")
    spec.datasource("memory")

    @spec.object_type(datasource="memory", primary_key="id")
    class Customer:
        id: str
        email: str | None

    @spec.trait
    class Contact:
        email: str

    @spec.link(from_type="customer", to_type="customer")
    class Referral:
        pass

    @spec.action(subject="customer")
    def welcome(email: str) -> str:
        return email

    @spec.policy(roles=["admin"], operations=["execute"])
    def welcome_policy():
        pass

    spec.add("roles", {"api_name": "admin", "inherits": []})
    spec.add("object_sets", {"api_name": "customers", "object_type": "customer", "sort": []})
    return spec


def test_complete_spec_round_trip():
    spec = sample_spec()
    data = spec.to_dict()
    assert set(data) == {"version", "workspace", "datasources", "object_types", "traits", "link_types", "actions", "policies", "roles", "object_sets"}
    assert Spec.from_json(spec.to_json()).to_dict() == data
    assert tool_definitions()


def test_advanced_fields_and_unknown_field():
    data = sample_spec().to_dict()
    data["workspace"].update(display="Demo", metadata={"owner": "team"})
    data["datasources"][0].update(config={"url": "local"}, secrets={"key": "vault/ref"})
    data["object_types"][0].update(display="Customer", description="Customers", tags=["crm"], metadata={"ui": "table"}, indexes=[{"api_name": "by_email", "properties": ["email"], "unique": True}])
    data["object_types"][0]["properties"][0].update(indexed=True, unique=True, default="missing", metadata={"note": "key"})
    data["object_types"][0]["properties"].append({"api_name": "embedding", "data_type": {"vector": 3}})
    data["link_types"][0].update(source={"datasource": "memory", "resource": "referrals"}, mappings=[{"from_property": "id", "to_property": "id"}])
    data["actions"][0].update(risk_level="low", output_schema={"type": "string"}, metadata={"owner": "team"})
    data["roles"][0].update(display="Admin", description="Operator")
    data["policies"][0].update(resource_kind="action", resource="welcome", priority=1, redactions=["email"])
    data["object_sets"][0].update(display="Customers", filter={"op": "eq", "field": "id", "value": "1"}, metadata={"owner": "team"})
    spec = Spec.from_dict(data)
    assert Spec.from_json(spec.to_json()).to_dict() == data
    data["actions"][0]["mistyped_field"] = "value"
    with pytest.raises(ValueError, match="mistyped_field"):
        Spec.from_dict(data).to_json()
    del data["actions"][0]["mistyped_field"]
    data["version"] = "tesela.spec.v2"
    with pytest.raises(ValueError, match="unsupported spec version"):
        Spec.from_dict(data).to_json()
    data["version"] = "tesela.spec.v1"
    data["object_types"][0]["api_name"] = "NotValid"
    with pytest.raises(ValueError, match="validation failed"):
        Spec.from_dict(data).to_json()


def test_native_memory_runtime():
    spec = sample_spec()
    runtime = Runtime(spec, stores={"memory": MemoryStore()}, policy="allow_all")
    runtime.mutate("customer", {"create": {"values": {"id": "1", "email": "a@example.com"}}}, actor=ACTOR)
    assert runtime.get("customer", "1", actor=ACTOR)["values"]["email"] == "a@example.com"
    assert len(runtime.search("customer", actor=ACTOR)["records"]) == 1
    assert runtime.resolve_object_set("customers", actor=ACTOR)["records"]
    assert runtime.compose_object_sets(["customers"], "union", actor=ACTOR)["records"]
    assert runtime.aggregate("customer", actor=ACTOR, query={"aggregations": [{"function": "count", "alias": "n"}]})["groups"][0]["n"] == 1
    runtime.mutate("customer", {"delete": {"primary_key": "1"}}, actor=ACTOR)
    assert runtime.search("customer", actor=ACTOR)["records"] == []


def test_limits_and_cross_language_defaults():
    spec = Spec()
    spec.datasource("memory")

    @spec.object_type(datasource="memory", primary_key="id")
    class CustomerOrder:
        id: str

    @spec.action(subject="customer_order")
    def create_order(score: float) -> float:
        return score

    data = spec.to_dict()
    assert data["object_types"][0]["api_name"] == "customer_order"
    assert data["object_types"][0]["source"]["resource"] == "customer_order"
    assert data["actions"][0]["risk_level"] == "low"
    assert data["actions"][0]["input_schema"]["properties"]["score"]["type"] == "number"

    @spec.action(subject="customer_order", input_schema={"type": "object", "properties": {"score": {"type": "number", "minimum": 0}}})
    def scored_order(score: float) -> float:
        return score

    assert spec.to_dict()["actions"][1]["input_schema"]["properties"]["score"]["minimum"] == 0
    runtime = Runtime(spec, stores={"memory": MemoryStore()}, policy="allow_all", max_query_limit=1)
    for id_ in ("1", "2"):
        runtime.mutate("customer_order", {"create": {"values": {"id": id_}}}, actor=ACTOR)
    assert len(runtime.search("customer_order", actor=ACTOR, query={"limit": 10})["records"]) == 1
    with pytest.raises(RuntimeError, match="max_query_limit must be positive"):
        Runtime(spec, stores={"memory": MemoryStore()}, policy="allow_all", max_query_limit=0)


class CallbackStore:
    def __init__(self):
        self.rows = {}

    def search(self, object_type, query):
        return {"records": list(self.rows.values()), "next_cursor": None}

    def get(self, object_type, primary_key):
        return self.rows.get(primary_key)

    def create(self, object_type, values):
        row = {"primary_key": values["id"], "values": values}
        self.rows[values["id"]] = row
        return {"record": row, "rows_affected": 1}

    def update(self, object_type, primary_key, values):
        self.rows[primary_key]["values"].update(values)
        return {"record": self.rows[primary_key], "rows_affected": 1}

    def delete(self, object_type, primary_key):
        return {"record": self.rows.pop(primary_key), "rows_affected": 1}

    def execute_action(self, request):
        return {"status": "success", "output": request["input"]}

    def aggregate(self, object_type, query):
        return {"groups": [{"n": len(self.rows)}]}

    def traverse(self, link_type, query):
        return {"records": list(self.rows.values())}


class Policy:
    def __init__(self, allow):
        self.allow = allow
        self.calls = []

    def evaluate(self, request):
        self.calls.append(request)
        return {"allow": self.allow, "reason": "blocked" if not self.allow else None}


class Audit:
    def __init__(self):
        self.events = []

    def record(self, event):
        self.events.append(event)


class Events:
    def __init__(self):
        self.events = []

    def publish(self, event):
        self.events.append(event)


def test_python_ports_actions_and_denial():
    spec = sample_spec()
    store, policy, audit, events = CallbackStore(), Policy(True), Audit(), Events()
    runtime = Runtime(spec, stores={"memory": store}, policy=policy, audit=audit, events=events)
    runtime.mutate("customer", {"create": {"values": {"id": "1"}}}, actor=ACTOR)
    assert runtime.execute_action("welcome", {"email": "hi"}, actor=ACTOR)["output"] == {"email": "hi"}
    assert runtime.traverse("referral", {"source_pk": "1"}, actor=ACTOR)["records"]
    assert runtime.aggregate("customer", actor=ACTOR)["groups"][0]["n"] == 1
    assert len(audit.events) == len(events.events) == 4
    assert any(call["operation"] == "execute" for call in policy.calls)
    policy.allow = False
    with pytest.raises(RuntimeError, match="policy denied"):
        runtime.get("customer", "1", actor=ACTOR)


def test_callback_errors_and_subject_requirement():
    spec = sample_spec()
    spec.add("actions", {"api_name": "free", "handler": {"kind": "callback"}})
    runtime = Runtime(spec, stores={"memory": CallbackStore()}, policy="allow_all")
    with pytest.raises(RuntimeError, match="has no subject"):
        runtime.execute_action("free", {}, actor=ACTOR)
    class BrokenStore(CallbackStore):
        def get(self, object_type, primary_key):
            raise ValueError("backend failed")

    broken = Runtime(spec, stores={"memory": BrokenStore()}, policy="allow_all")
    with pytest.raises(RuntimeError, match="Python get.*backend failed"):
        broken.get("customer", "unknown", actor=ACTOR)


def test_parallel_reads_release_gil_and_reenter_python_ports():
    spec = sample_spec()
    store = CallbackStore()
    store.create("customer", {"id": "1"})
    runtime = Runtime(spec, stores={"memory": store}, policy=Policy(True))
    with ThreadPoolExecutor(max_workers=4) as workers:
        rows = list(workers.map(lambda _: runtime.get("customer", "1", actor=ACTOR), range(100)))
    assert all(row["primary_key"] == "1" for row in rows)


def test_required_port_methods_fail_at_construction():
    with pytest.raises(TypeError, match="missing required methods"):
        Runtime(sample_spec(), stores={"memory": object()}, policy="allow_all")
    with pytest.raises(TypeError, match="policy must implement"):
        Runtime(sample_spec(), stores={"memory": MemoryStore()}, policy=None)


def test_policy_row_filter_and_redaction_cross_binding():
    class FilteringPolicy:
        def evaluate(self, request):
            return {"allow": True, "row_filter": {"op": "eq", "field": "id", "value": "1"}, "redactions": ["email"]}

    runtime = Runtime(sample_spec(), stores={"memory": MemoryStore()}, policy=FilteringPolicy())
    for id_ in ("1", "2"):
        runtime.mutate("customer", {"create": {"values": {"id": id_, "email": f"{id_}@example.com"}}}, actor=ACTOR)
    page = runtime.search("customer", actor=ACTOR)
    assert [row["primary_key"] for row in page["records"]] == ["1"]
    assert "email" not in page["records"][0]["values"]

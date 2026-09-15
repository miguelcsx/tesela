"""Reproducible Python entrypoint over the same native MemoryStore workload."""

from time import perf_counter

from tesela import MemoryStore, Runtime, Spec


def measure(label, work, count=10_000):
    start = perf_counter()
    for _ in range(count):
        work()
    print(f"{label}: {(perf_counter() - start) * 1_000_000 / count:.3f} us/op ({count} ops)")


def main():
    spec = Spec()
    spec.datasource("memory")

    @spec.object_type(datasource="memory", primary_key="id")
    class Customer:
        id: str
        score: int

    actor = {"user_id": "bench"}
    runtime = Runtime(spec, stores={"memory": MemoryStore()}, policy="allow_all")
    for number in range(100):
        runtime.mutate("customer", {"create": {"values": {"id": str(number), "score": number}}}, actor=actor)
    document = spec.to_json()
    update = {"upsert": {"values": {"id": "0", "score": 1}}}
    measure("spec.parse", lambda: Spec.from_json(document))
    measure("spec.serialize", spec.to_json)
    measure("search.100", lambda: runtime.search("customer", actor=actor))
    measure("get", lambda: runtime.get("customer", "0", actor=actor))
    measure("mutate.upsert", lambda: runtime.mutate("customer", update, actor=actor))


if __name__ == "__main__":
    main()

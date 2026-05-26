import os

import pytest

from tesela import App, NativeRuntime, String


def test_native_runtime_search_with_callback_backend():
    lib = os.environ.get("TESELA_NATIVE_LIB")
    if not lib:
        pytest.skip("TESELA_NATIVE_LIB is not set")

    app = App("native")
    app.datasource("main", "python")
    app.object_type("customer") \
        .source("main", "customers") \
        .property("id", String) \
        .property("email", String) \
        .primary_key("id") \
        .done()

    def backend(req):
        if req["op"] == "capabilities":
            return {"search": {"enabled": True}, "get": True}
        if req["op"] == "search":
            return {
                "records": [
                    {"primary_key": "c1", "values": {"id": "c1", "email": "a@example.com"}},
                ],
                "total_count": 1,
            }
        raise AssertionError(req["op"])

    with NativeRuntime.from_app(app, library_path=lib) as rt:
        rt.register_backend("python", backend)
        page = rt.search("customer", {"limit": 10})

    assert page["records"][0]["values"]["email"] == "a@example.com"


def test_native_runtime_callback_error_propagates():
    lib = os.environ.get("TESELA_NATIVE_LIB")
    if not lib:
        pytest.skip("TESELA_NATIVE_LIB is not set")

    app = App("native")
    app.datasource("main", "python")
    app.object_type("customer") \
        .source("main", "customers") \
        .property("id", String) \
        .primary_key("id") \
        .done()

    def backend(req):
        if req["op"] == "capabilities":
            return {"search": {"enabled": True}, "get": True}
        raise RuntimeError("backend exploded")

    with NativeRuntime.from_app(app, library_path=lib) as rt:
        rt.register_backend("python", backend)
        with pytest.raises(Exception) as exc:
            rt.search("customer", {"limit": 10})

    assert "backend exploded" in str(exc.value)

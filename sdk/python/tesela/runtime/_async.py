from __future__ import annotations

import asyncio
import functools
import os
from typing import Any

from tesela.runtime._sync import NativeRuntime


class AsyncNativeRuntime:

    def __init__(self, runtime: NativeRuntime):
        self._rt = runtime

    @classmethod
    async def from_app(
        cls, app: Any, *, library_path: str | os.PathLike[str] | None = None
    ) -> AsyncNativeRuntime:
        rt = await asyncio.to_thread(NativeRuntime.from_app, app, library_path=library_path)
        return cls(rt)

    @classmethod
    async def from_spec(
        cls,
        spec: dict[str, Any] | str | bytes,
        *,
        library_path: str | os.PathLike[str] | None = None,
    ) -> AsyncNativeRuntime:
        rt = await asyncio.to_thread(lambda: NativeRuntime(spec, library_path=library_path))
        return cls(rt)

    def __getattr__(self, name: str):
        attr = getattr(self._rt, name)
        if not callable(attr):
            return attr

        @functools.wraps(attr)
        async def _async(*args, **kwargs):
            return await asyncio.to_thread(attr, *args, **kwargs)

        return _async

    def __repr__(self) -> str:
        return f"AsyncNativeRuntime(handle={self._rt._handle})"

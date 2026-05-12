"""M0 smoke tests — make sure scaffold imports + healthz works."""

from __future__ import annotations

import pytest
from fastapi.testclient import TestClient

from taskboard import __version__
from taskboard.main import create_app


def test_version_is_string() -> None:
    assert isinstance(__version__, str)
    assert __version__.startswith("0.")


def test_healthz() -> None:
    app = create_app()
    client = TestClient(app)
    r = client.get("/healthz")
    assert r.status_code == 200
    body = r.json()
    assert body["status"] == "ok"
    assert body["milestone"] == "M0"


@pytest.mark.asyncio
async def test_llm_router_no_key_raises() -> None:
    """M3 stub — without env key, call_with_schema must raise LLMError."""
    import os

    from pydantic import BaseModel

    from taskboard.llm_router import LLMError, call_with_schema

    # Clear keys
    os.environ.pop("ANTHROPIC_API_KEY", None)
    os.environ.pop("OPENAI_API_KEY", None)

    class Out(BaseModel):
        tags: list[str]

    with pytest.raises(LLMError, match="no API key"):
        await call_with_schema(prompt="hi", schema=Out)

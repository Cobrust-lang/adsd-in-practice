"""LLM router — Anthropic + OpenAI-compatible.

Borrowed schema design from Cobrust Studio's `studio-router` crate
(ADR-0005 in this case). Single async entry-point that picks the
provider based on env vars and returns a typed pydantic model.

Wave M3 stub. Real implementation lands at M3.1.
"""

from __future__ import annotations

import os
from typing import TypeVar

from pydantic import BaseModel

T = TypeVar("T", bound=BaseModel)


class LLMError(Exception):
    """Raised on router / provider failure."""


async def call_with_schema(  # noqa: D401  (imperative)
    *,
    prompt: str,
    schema: type[T],
    max_retries: int = 1,
) -> T:
    """Call the configured LLM and validate output against `schema`.

    Picks the provider from env:
    - `ANTHROPIC_API_KEY` set → Anthropic claude-haiku-* default
    - `OPENAI_API_KEY` set → OpenAI gpt-* compatible

    On `pydantic.ValidationError`, retry up to `max_retries` times with
    "Your previous response did not match the schema; try again. Errors: ..."
    appended.

    M3.1 stub — real impl lands when LLM router is lifted from
    cobrust-studio/studio-router.
    """
    _ = prompt
    _ = schema
    _ = max_retries
    if not (os.environ.get("ANTHROPIC_API_KEY") or os.environ.get("OPENAI_API_KEY")):
        raise LLMError(
            "no API key in env (set ANTHROPIC_API_KEY or OPENAI_API_KEY)"
        )
    raise NotImplementedError("M3.1 stub — wire up real provider call")

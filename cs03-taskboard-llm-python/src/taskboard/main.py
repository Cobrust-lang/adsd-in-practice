"""FastAPI app entry point.

Wave M0 scaffold — actual routes land at Wave M1.
"""

from __future__ import annotations

import logging
import os

from fastapi import FastAPI

logger = logging.getLogger(__name__)


def create_app() -> FastAPI:
    """Build the FastAPI app.

    Wave M1 will register routers from `taskboard.api.{tasks, auto_tag}`.
    """
    app = FastAPI(
        title="taskboard",
        description="ADSD CS-03 — LLM-augmented task board (M0 scaffold)",
        version="0.1.0",
    )

    @app.get("/healthz")
    async def healthz() -> dict[str, str]:
        return {"status": "ok", "milestone": "M0"}

    return app


app = create_app()


def run() -> None:
    """Console-script entry point (`taskboard` shell command)."""
    import uvicorn

    port = int(os.environ.get("PORT", "8000"))
    logger.info("starting taskboard on :%d (M0 scaffold)", port)
    uvicorn.run("taskboard.main:app", host="0.0.0.0", port=port, reload=False)

from collections.abc import AsyncGenerator

import httpx
import pytest
import src.main as main_mod
from src.cache import CachedEmbedder
from src.main import app


@pytest.fixture
async def test_client() -> AsyncGenerator[httpx.AsyncClient, None]:
    """Yield an httpx AsyncClient with lifespan startup run."""
    # Manually trigger lifespan startup (set up embedder)
    main_mod.embedder = CachedEmbedder()
    try:
        transport = httpx.ASGITransport(app=app)
        async with httpx.AsyncClient(transport=transport, base_url="http://test") as client:
            yield client
    finally:
        main_mod.embedder = None


@pytest.fixture
def sample_texts() -> list[str]:
    """Return a small list of sample texts for testing."""
    return [
        "What is the capital of France?",
        "How does gradient descent work?",
        "Explain the transformer architecture.",
    ]

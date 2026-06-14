import shutil
import tempfile
from collections.abc import Generator

import pytest
from fastapi.testclient import TestClient

from src.cache import CachedEmbedder
from src.service import EmbeddingService


@pytest.fixture
def cache_dir() -> str:
    """Create a temporary directory for cache tests."""
    return tempfile.mkdtemp()


class TestHealthEndpoint:
    """Tests for GET /health."""

    def test_health_endpoint(self, test_client: TestClient) -> None:
        response = test_client.get("/health")
        assert response.status_code == 200
        assert response.json() == {"status": "ok"}


class TestEmbedEndpoint:
    """Tests for POST /embed."""

    def test_embed_endpoint(self, test_client: TestClient, sample_texts: list[str]) -> None:
        response = test_client.post("/embed", json={"texts": sample_texts})
        assert response.status_code == 200
        data = response.json()
        assert "embeddings" in data
        assert isinstance(data["embeddings"], list)
        assert len(data["embeddings"]) == len(sample_texts)
        assert all(isinstance(emb, list) for emb in data["embeddings"])
        assert all(isinstance(val, float) for emb in data["embeddings"] for val in emb)
        assert "model" in data
        assert isinstance(data["model"], str)

    def test_embed_empty_list(self, test_client: TestClient) -> None:
        response = test_client.post("/embed", json={"texts": []})
        assert response.status_code == 200
        data = response.json()
        assert data["embeddings"] == []


class TestCache:
    """Unit tests for cache hit/miss behaviour."""

    def test_cache_hit_and_miss(self, cache_dir: str) -> None:
        texts_a = ["hello world"]
        texts_b = ["different text"]

        service = EmbeddingService()
        cached = CachedEmbedder(service=service, cache_dir=cache_dir)

        # First call — should miss
        emb_a = cached.embed(texts_a)
        assert cached.hits == 0
        assert cached.misses == 1

        # Second call with same texts — should hit
        emb_a2 = cached.embed(texts_a)
        assert cached.hits == 1
        assert cached.misses == 1
        assert emb_a == emb_a2

        # Different texts — should miss again
        emb_b = cached.embed(texts_b)
        assert cached.hits == 1
        assert cached.misses == 2
        assert emb_a != emb_b

    @pytest.fixture(autouse=True)
    def _cleanup(self, cache_dir: str) -> Generator[None, None, None]:
        yield

        shutil.rmtree(cache_dir, ignore_errors=True)

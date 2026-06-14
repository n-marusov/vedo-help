from collections.abc import Generator

import pytest
from fastapi.testclient import TestClient

from src.main import app


@pytest.fixture
def test_client() -> Generator[TestClient, None, None]:
    """Yield a FastAPI TestClient instance."""
    with TestClient(app) as client:
        yield client


@pytest.fixture
def sample_texts() -> list[str]:
    """Return a small list of sample texts for testing."""
    return [
        "What is the capital of France?",
        "How does gradient descent work?",
        "Explain the transformer architecture.",
    ]

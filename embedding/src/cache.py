import os
from typing import cast, final

import diskcache
import structlog

from src.service import EmbeddingService

logger = structlog.get_logger(__name__)

DEFAULT_CACHE_DIR = "/data/cache"


@final
class CachedEmbedder:
    """Wraps EmbeddingService with a disk-based cache."""

    def __init__(
        self,
        service: EmbeddingService | None = None,
        cache_dir: str | None = None,
    ) -> None:
        self._service = service or EmbeddingService()
        self._cache_dir = cache_dir or os.environ.get("CACHE_DIR", DEFAULT_CACHE_DIR)
        self._cache: diskcache.Cache = diskcache.Cache(self._cache_dir)
        self._hits = 0
        self._misses = 0

    @property
    def hits(self) -> int:
        return self._hits

    @property
    def misses(self) -> int:
        return self._misses

    @property
    def model_name(self) -> str:
        return self._service.model_name

    def get(self, key: str) -> list[list[float]] | None:
        """Return cached embedding for *key*, or None."""
        value = cast("list[list[float]] | None", self._cache.get(key))
        if value is not None:
            self._hits += 1
            logger.info("cache.hit", key=key, hits=self._hits, misses=self._misses)
        else:
            self._misses += 1
            logger.info("cache.miss", key=key, hits=self._hits, misses=self._misses)
        return value

    def set(self, key: str, value: list[list[float]]) -> None:
        """Store *value* in the cache under *key*."""
        self._cache.set(key, value)

    def embed(self, texts: list[str]) -> list[list[float]]:
        """Embed texts using cache when possible."""
        key = "\x00".join(texts)
        cached = self.get(key)
        if cached is not None:
            return cached
        embeddings = self._service.embed(texts)
        self.set(key, embeddings)
        return embeddings

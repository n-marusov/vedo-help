import logging
import os

from sentence_transformers import SentenceTransformer

logger = logging.getLogger(__name__)

DEFAULT_MODEL = "BAAI/bge-small-en-v1.5"


class EmbeddingService:
    """Encapsulates a sentence-transformers model for text embedding."""

    def __init__(self, model_name: str | None = None) -> None:
        self._model_name = model_name or os.environ.get("EMBEDDING_MODEL", DEFAULT_MODEL)
        logger.debug("Loading model: %s", self._model_name)
        self._model: SentenceTransformer = SentenceTransformer(self._model_name)
        logger.debug("Model loaded: %s", self._model_name)

    @property
    def model_name(self) -> str:
        return self._model_name

    def embed(self, texts: list[str]) -> list[list[float]]:
        """Compute embeddings for a list of input texts."""
        logger.debug("Embedding %d text(s)", len(texts))
        embeddings = self._model.encode(texts, show_progress_bar=False)
        return embeddings.tolist()  # type: ignore[union-attr]

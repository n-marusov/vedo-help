import logging

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from src.cache import CachedEmbedder
from src.models import EmbedRequest, EmbedResponse

logger = logging.getLogger(__name__)

app = FastAPI(title="VEDO Embedding Service", version="0.1.0")

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

embedder: CachedEmbedder | None = None


@app.on_event("startup")
async def on_startup() -> None:
    """Initialise the embedder and configure logging on startup."""
    logging.basicConfig(
        level=logging.DEBUG,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )
    logger.debug("Starting VEDO Embedding Service")

    global embedder  # noqa: PLW0603
    embedder = CachedEmbedder()
    logger.debug("Embedder ready (model=%s)", embedder.model_name)


@app.get("/health")
async def health() -> dict[str, str]:
    """Liveness probe."""
    return {"status": "ok"}


@app.post("/embed")
async def embed(request: EmbedRequest) -> EmbedResponse:
    """Compute embeddings for one or more input texts."""
    if embedder is None:
        msg = "Embedder not initialized"
        raise RuntimeError(msg)
    embeddings = embedder.embed(request.texts)
    return EmbedResponse(embeddings=embeddings, model=embedder.model_name)

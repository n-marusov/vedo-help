import logging
import os
from collections.abc import AsyncGenerator

import structlog
from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware
from opentelemetry.exporter.otlp.proto.grpc._log_exporter import OTLPLogExporter
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.sdk._logs import LoggerProvider, LoggingHandler
from opentelemetry.sdk._logs.export import BatchLogRecordProcessor
from opentelemetry.sdk.resources import Resource

from src.cache import CachedEmbedder
from src.models import EmbedRequest, EmbedResponse

logger = structlog.get_logger(__name__)

embedder: CachedEmbedder | None = None

SERVICE_NAME = "vedo-embedding"
SERVICE_VERSION = "0.1.0"
OTEL_ENDPOINT = os.environ.get("OTEL_EXPORTER_OTLP_ENDPOINT", "")


def init_telemetry() -> LoggerProvider | None:
    """Initialize OpenTelemetry logging and structlog.

    Uses structlog 25.x native API (ProcessorFormatter bridge removed).
    For OTel export, adds a stdlib LoggingHandler that captures logs from
    libraries using stdlib logging.
    """
    resource = Resource.create(
        {
            "service.name": SERVICE_NAME,
            "service.version": SERVICE_VERSION,
            "deployment.environment": os.environ.get("ENVIRONMENT", "development"),
        },
    )

    logger_provider: LoggerProvider | None = None
    env = os.environ.get("ENVIRONMENT", "development")

    processors = [
        structlog.contextvars.merge_contextvars,
        structlog.stdlib.filter_by_level,
        structlog.stdlib.add_logger_name,
        structlog.stdlib.add_log_level,
        structlog.stdlib.PositionalArgumentsFormatter(),
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.format_exc_info,
        structlog.dev.set_exc_info,
    ]

    if env != "development":
        processors.append(structlog.processors.JSONRenderer())
    else:
        processors.append(structlog.dev.ConsoleRenderer())

    structlog.configure(
        processors=processors,
        wrapper_class=structlog.stdlib.BoundLogger,
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),
        cache_logger_on_first_use=True,
    )

    if OTEL_ENDPOINT:
        logger_provider = LoggerProvider(resource=resource)
        exporter = OTLPLogExporter(endpoint=OTEL_ENDPOINT, insecure=True)
        logger_provider.add_log_record_processor(BatchLogRecordProcessor(exporter))

        # Bridge stdlib logging to OTel (catches libraries that use stdlib logging)
        otel_handler = LoggingHandler(level=None, logger_provider=logger_provider)
        logging.getLogger().addHandler(otel_handler)

        structlog.get_logger(__name__).info(
            "otel.initialized",
            endpoint=OTEL_ENDPOINT,
            service_name=SERVICE_NAME,
            environment=env,
        )
    else:
        structlog.get_logger(__name__).info(
            "telemetry.initialized",
            service_name=SERVICE_NAME,
            environment=env,
        )

    return logger_provider


_otel_logger_provider: LoggerProvider | None = None


async def lifespan(_app: FastAPI) -> AsyncGenerator[None, None]:
    """Initialise telemetry and the embedder on startup."""
    global _otel_logger_provider, embedder  # noqa: PLW0603

    _otel_logger_provider = init_telemetry()

    logger.info("embedding.startup")
    embedder = CachedEmbedder()
    logger.info("embedding.model_loaded", model=embedder.model_name)
    yield
    logger.info("embedding.shutdown")
    if _otel_logger_provider is not None:
        _otel_logger_provider.shutdown()


app = FastAPI(title="VEDO Embedding Service", version="0.1.0", lifespan=lifespan)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

FastAPIInstrumentor.instrument_app(app)


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
    logger.debug("embed.request", text_count=len(request.texts))
    embeddings = embedder.embed(request.texts)
    return EmbedResponse(embeddings=embeddings, model=embedder.model_name)

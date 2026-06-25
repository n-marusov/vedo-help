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

    Sets up:
    1. OTel LoggerProvider with Resource attributes and OTLP export
    2. structlog with JSON console output and OTel forwarding
    3. stdlib LoggingHandler bridge for libraries using stdlib logging
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

    shared_processors = [
        structlog.contextvars.merge_contextvars,
        structlog.stdlib.filter_by_level,
        structlog.stdlib.add_logger_name,
        structlog.stdlib.add_log_level,
        structlog.stdlib.PositionalArgumentsFormatter(),
        structlog.processors.TimeStamper(fmt="iso"),
        structlog.processors.StackInfoRenderer(),
        structlog.processors.format_exc_info,
        structlog.dev.set_exc_info,
        structlog.stdlib.ProcessorFormatter.wrap_for_instrumentation(),
    ]

    if OTEL_ENDPOINT:
        logger_provider = LoggerProvider(resource=resource)
        exporter = OTLPLogExporter(endpoint=OTEL_ENDPOINT, insecure=True)
        logger_provider.add_log_record_processor(BatchLogRecordProcessor(exporter))

        # Bridge stdlib logging to OTel (catches libraries that use stdlib logging)
        otel_handler = LoggingHandler(level=None, logger_provider=logger_provider)
        logging.getLogger().addHandler(otel_handler)

        structlog.configure(
            processors=shared_processors,
            wrapper_class=structlog.stdlib.BoundLogger,
            context_class=dict,
            logger_factory=structlog.stdlib.LoggerFactory(),
            cache_logger_on_first_use=True,
        )

        proc_processor = structlog.stdlib.ProcessorFormatter(
            processor=structlog.processors.JSONRenderer()
            if env != "development"
            else structlog.dev.ConsoleRenderer(),
        )
        handler = logging.StreamHandler()
        handler.setFormatter(proc_processor)
        root_logger = logging.getLogger()
        root_logger.addHandler(handler)
        root_logger.setLevel(logging.DEBUG)

        structlog.stdlib.ProcessorFormatter.remove_instrumentation()

        structlog.get_logger(__name__).info(
            "otel.initialized",
            endpoint=OTEL_ENDPOINT,
            service_name=SERVICE_NAME,
            environment=env,
        )
    else:
        # No OTel endpoint — simple structlog console output only
        structlog.configure(
            processors=shared_processors,
            wrapper_class=structlog.stdlib.BoundLogger,
            context_class=dict,
            logger_factory=structlog.stdlib.LoggerFactory(),
            cache_logger_on_first_use=True,
        )

        proc_processor = structlog.stdlib.ProcessorFormatter(
            processor=structlog.dev.ConsoleRenderer(),
        )
        handler = logging.StreamHandler()
        handler.setFormatter(proc_processor)
        root_logger = logging.getLogger()
        root_logger.addHandler(handler)
        root_logger.setLevel(logging.DEBUG)

        structlog.stdlib.ProcessorFormatter.remove_instrumentation()

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

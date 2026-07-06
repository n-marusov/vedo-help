# ruff: noqa: INP001
from collections.abc import Iterator  # noqa: TC003
from typing import Any

from fastapi import FastAPI, Request, Response
from fastapi.responses import JSONResponse, StreamingResponse

app = FastAPI()


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


EMBEDDING_DIM = 384


@app.post("/embeddings")
async def embeddings(request: Request) -> JSONResponse:
    """Mock OpenAI-compatible embeddings endpoint.

    Returns fixed 384-dimensional vectors (matching all-MiniLM-L6-v2)
    so that the RAG pipeline tests can run without an external API.
    """
    body: dict[str, Any] = await request.json()
    inputs: list[str] = body.get("input", [])
    if isinstance(inputs, str):
        inputs = [inputs]

    data = [
        {
            "object": "embedding",
            "index": i,
            "embedding": [0.01 * (j % 10) for j in range(EMBEDDING_DIM)],
        }
        for i in range(len(inputs))
    ]

    return JSONResponse(
        {
            "object": "list",
            "data": data,
            "model": body.get("model", "sentence-transformers/all-minilm-l6-v2"),
            "usage": {
                "prompt_tokens": sum(len(t.split()) for t in inputs),
                "total_tokens": sum(len(t.split()) for t in inputs),
            },
        }
    )


@app.post("/chat/completions", response_model=None)
async def chat_completions(request: Request) -> Response:
    """Mock LLM endpoint compatible with OpenAI chat completions format.

    Supports both streaming (SSE) and non-streaming (JSON) responses.
    Detects the intended pipeline stage from the system prompt content.
    """
    body: dict[str, Any] = await request.json()
    is_stream: bool = body.get("stream", False)

    messages = body.get("messages", [])
    system_prompt = ""
    user_prompt = ""
    for msg in messages:
        if msg.get("role") == "system":
            system_prompt = msg.get("content", "")
        elif msg.get("role") == "user":
            user_prompt = msg.get("content", "")

    # Select response content based on pipeline stage
    content = _select_response(system_prompt, user_prompt)

    if is_stream:
        chunks: Iterator[str] = iter(
            [
                f'data: {{"choices":[{{"delta":{{"content":"{content} "}}}}]}}\n\n',
                "data: [DONE]\n\n",
            ],
        )
        return StreamingResponse(chunks, media_type="text/event-stream")
    return JSONResponse(
        {
            "id": "mock-chatcmpl-001",
            "object": "chat.completion",
            "created": 1700000000,
            "model": "mock-model",
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": content,
                    },
                    "finish_reason": "stop",
                },
            ],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15,
            },
        },
    )


def _select_response(system_prompt: str, user_prompt: str) -> str:  # noqa: ARG001
    """Return appropriate mock content based on detected pipeline stage."""
    sp_lower = system_prompt.lower()

    # Reranking: detect "relevance ranker" or "брать"/"пропустить"
    if (
        "relevance ranker" in sp_lower
        or "брать" in sp_lower
        or "пропустить" in sp_lower
    ):
        return "брать"

    # Multi-Query: detect "alternative questions" or "multiple perspectives"
    if (
        "alternative questions" in sp_lower
        or "multiple perspectives" in sp_lower
        or "multi" in sp_lower
    ):
        return "What is the VEDO hub platform?\nHow does VEDO hub work?\nDescribe VEDO hub features."

    # HyDE: detect "hypothetical document"
    if "hypothetical document" in sp_lower:
        return (
            "VEDO hub is a RAG (Retrieval-Augmented Generation) assistant system "
            "that ingests technical documentation and answers questions using "
            "vector search and large language models. It supports PDF, Markdown, "
            "and DOCX file upload, Chroma vector database indexing, and streaming "
            "LLM responses with grounded citations."
        )

    # Standard chat: return the mock answer used by the streaming path
    return "Test backend answer from indexed documents. Sources are attached by the backend."

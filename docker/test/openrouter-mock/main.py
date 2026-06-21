from typing import TYPE_CHECKING

from fastapi import FastAPI
from fastapi.responses import StreamingResponse

if TYPE_CHECKING:
    from collections.abc import Iterator

app = FastAPI()


@app.get("/health")
def health() -> dict[str, str]:
    return {"status": "ok"}


@app.post("/chat/completions")
def chat_completions() -> StreamingResponse:
    chunks: Iterator[str] = iter(
        [
            'data: {"choices":[{"delta":{"content":"Test backend answer from indexed documents. "}}]}\n\n',
            'data: {"choices":[{"delta":{"content":"Sources are attached by the backend."}}]}\n\n',
            "data: [DONE]\n\n",
        ],
    )
    return StreamingResponse(chunks, media_type="text/event-stream")

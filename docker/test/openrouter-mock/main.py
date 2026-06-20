from fastapi import FastAPI
from fastapi.responses import StreamingResponse

app = FastAPI()


@app.get("/health")
def health():
    return {"status": "ok"}


@app.post("/chat/completions")
def chat_completions():
    chunks = [
        'data: {"choices":[{"delta":{"content":"Test backend answer from indexed documents. "}}]}\n\n',
        'data: {"choices":[{"delta":{"content":"Sources are attached by the backend."}}]}\n\n',
        "data: [DONE]\n\n",
    ]
    return StreamingResponse(iter(chunks), media_type="text/event-stream")

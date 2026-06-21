FROM python:3.13-slim

WORKDIR /app
RUN pip install --no-cache-dir fastapi uvicorn
COPY main.py /app/main.py
EXPOSE 18002
CMD ["uvicorn", "main:app", "--host", "0.0.0.0", "--port", "18002"]

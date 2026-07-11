# Web Crawler & Site Ingestion

> Feature overview for ingesting documents from websites via a BFS web crawler.

## Overview

The Web Crawler module adds a third document source alongside manual upload and Git sync: crawl any website from an entry URL, extract clean text content, chunk it, embed it, and index it into Chroma — all in one background pipeline.

The crawler uses **BFS (Breadth-First Search)** with four layers of safeguards to prevent accidental mass crawling:

1. **Same-domain enforcement** — never leaves the domain of the entry URL
2. **Path prefix scoping** — optional filter to restrict crawling to a URL prefix
3. **Max pages** — hard cap (default 50, configurable 1–10000)
4. **Max depth** — BFS depth limit (default 2, configurable 1–10)

## Use Cases

- **Documentation sites** — index technical docs from `docs.example.com`
- **Knowledge bases** — crawl internal or public wikis
- **Blog aggregation** — collect posts from a blog with a predictable URL structure

## Configuration Parameters

| Parameter | Default | Range | Description |
|-----------|---------|-------|-------------|
| `max_depth` | 2 | 1–10 | Maximum BFS depth from entry URL |
| `max_pages` | 50 | 1–10000 | Maximum pages to crawl |
| `delay_ms` | 1000 | 100–10000 | Delay between requests (rate limiting) |
| `path_prefix` | (empty) | — | Optional URL prefix filter (e.g., `/docs`) |

## Limitations

- **No JavaScript rendering** — only static HTML content is extracted (SPAs are not supported)
- **Same-domain only** — the crawler never follows links to other domains
- **No authenticated content** — pages behind login are not accessible
- **No sitemap.xml auto-discovery** — the crawler uses BFS from the entry URL
- **No PDF/image crawling** — only HTML pages are processed
- **Respects robots.txt** — the crawler fetches and caches robots.txt per domain

## API Reference

### Create Crawl Job

```
POST /api/web-crawl
```

Creates a new crawl job and starts crawling immediately.

**Request body:**
```json
{
  "entry_url": "https://example.com/docs",
  "collection_id": "uuid",
  "config": {
    "max_depth": 2,
    "max_pages": 50,
    "delay_ms": 1000,
    "path_prefix": "/docs"
  }
}
```

**Response:** `CrawlJobSummary`

### List Crawl Jobs

```
GET /api/web-crawl
```

Lists all crawl jobs for the current user (admin sees all).

**Response:** `CrawlJobSummary[]`

### Get Job Detail

```
GET /api/web-crawl/:id
```

Returns the job details with the list of discovered pages.

**Response:** `CrawlJobDetailResponse`

### Cancel Job

```
POST /api/web-crawl/:id/cancel
```

Cancels a running crawl job. Sends a cancellation signal to the background task.

**Response:** `CrawlJobSummary`

### Delete Job

```
DELETE /api/web-crawl/:id
```

Deletes a crawl job and all associated pages.

**Response:** `{ "status": "deleted", "id": "uuid" }`

### Retry Failed Pages

```
POST /api/web-crawl/:id/retry
```

Resets all failed pages to pending status for re-crawling.

**Response:** `CrawlJobSummary`

### Subscribe to Progress (SSE)

```
GET /api/web-crawl/:id/subscribe
```

Streams real-time crawl progress as Server-Sent Events. Polls every 2 seconds and sends `CrawlStatusResponse` JSON. Stream ends when the job reaches a terminal state (`completed`, `cancelled`, `error`).

**Response:** SSE stream of `CrawlStatusResponse` events

## UI Guide

1. Navigate to **Admin Panel** → select a collection
2. Click the **Web Crawl** source tab
3. Click **+ New Crawl** to open the configuration dialog
4. Enter the **Entry URL** (must start with `http://` or `https://`)
5. Configure **Max Depth**, **Max Pages**, **Delay**, and optional **Path Prefix**
6. Click **Start Crawl** — the job appears in the job list
7. Monitor status: **idle** → **crawling** → **completed** / **cancelled** / **error**
8. Use **Cancel** to stop a running crawl, **Delete** to remove a completed job

## Troubleshooting

| Symptom | Likely Cause | Solution |
|---------|-------------|----------|
| Job stuck in `idle` | Concurrent crawl lock held | Wait or delete and recreate |
| Job status `error` | Network error or timeout | Check entry URL is reachable |
| No pages discovered | robots.txt blocking or wrong prefix | Check robots.txt and path prefix |
| Unexpected pages | No path prefix set | Add path prefix to limit scope |
| Slow crawl | Rate limiting delay | Reduce `delay_ms` in config |

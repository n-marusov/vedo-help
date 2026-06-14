[← Architecture](architecture.md) · [Back to README](../README.md) · [API Reference →](api.md)

# User Interface Guide

The frontend is a single-page application (SPA) built with Vue 3 and TypeScript. It has two main views accessed from the sidebar: **Chat** (Q&A interface) and **Admin** (collection and document management).

## Layout

The app shell consists of a dark-themed sidebar and a main content area.

```
┌──────────────┬──────────────────────────────────────┐
│  VEDO hub    │                                      │
│              │                                      │
│  💬 Chat     │         Main Content Area            │
│  ⚙️ Admin    │                                      │
│              │                                      │
│              │                                      │
│  v0.1.0      │                                      │
└──────────────┴──────────────────────────────────────┘
```

- **Sidebar** — navigation between Chat and Admin views. Displays the project name and version.
- **Main area** — renders the active view (chat interface or admin panel).

---

## Chat View (`/`)

The chat view is the primary Q&A interface. It has two panels: a session sidebar on the left and the chat window on the right.

### Session Sidebar

```
┌──────────────────────┐
│  SESSIONS     + New  │
├──────────────────────┤
│  ┌────────────────┐  │
│  │ How to config… │  │
│  │ 3 msg · 2h ago │🗑️│
│  └────────────────┘  │
│  ┌────────────────┐  │
│  │ API setup      │  │
│  │ 1 msg · 1d ago │🗑️│
│  └────────────────┘  │
└──────────────────────┘
```

| Element | Description |
|---------|-------------|
| **Sessions heading** | Section title with `+ New` button to start a fresh chat |
| **Session list** | Clickable sessions sorted by recency. Each shows title, message count, and relative time |
| **Active session** | Highlighted with a blue border |
| **Delete (🗑️)** | Appears on hover — removes the session after confirmation |

### Chat Window

```
┌───────────────────────────────────────────────┐
│  [Select a collection ▼]                   ✏️ │
├───────────────────────────────────────────────┤
│                                               │
│  💬  VEDO RAG Assistant                       │
│      Select a collection and ask a question.  │
│                                               │
│  ┌──────────────────────────────────────┐     │
│  │ You  2:30 PM                         │     │
│  │ How do I configure the rate limiter? │     │
│  └──────────────────────────────────────┘     │
│  ┌──────────────────────────────────────┐     │
│  │ 🤖 VEDO Assistant  2:30 PM           │     │
│  │ The rate limiter is configured via…  │     │
│  │                                      │     │
│  │ 📚 2 sources ▸                       │     │
│  └──────────────────────────────────────┘     │
│                                               │
├───────────────────────────────────────────────┤
│ │ Ask a question about your documents...   ➤ │
└───────────────────────────────────────────────┘
```

| Element | Description |
|---------|-------------|
| **Collection selector** | Dropdown at the top — choose which collection to query. Disabled state shows a placeholder prompt |
| **New chat (✏️)** | Clears the current conversation and creates a new session |
| **Messages area** | Scrollable list of user and assistant messages. Shows a welcome screen when empty |
| **Message bubbles** | User messages right-aligned (blue), assistant messages left-aligned (dark). Each shows role, timestamp, and formatted markdown content |
| **Sources section** | Collapsible panel under assistant messages showing cited documents, chunk text previews, and relevance scores |
| **Input area** | Textarea with auto-resize; supports Enter to send, Shift+Enter for newline |
| **Send button (➤)** | Submits the query. Shows a spinner during streaming |
| **Cancel (⏹)** | Appears during streaming — aborts the current LLM response |
| **Error banner** | Red highlighted message bar when the API returns an error |

### Streaming Flow

1. User types a question and presses Enter (or clicks Send)
2. A user message bubble appears immediately (optimistic update)
3. An empty assistant bubble appears with a typing indicator (three bouncing dots)
4. As the backend streams tokens via SSE, the assistant bubble fills in progressively
5. After completion, sources appear below the message
6. If the user clicks Cancel, streaming stops and the partial response is kept

### Message Sources

Click the **📚 N sources** toggle to expand source citations:

```
📚 2 sources ▾
┌──────────────────────────────────────┐
│ config-guide.md              92%    │
│ The rate limiter is configured by…  │
└──────────────────────────────────────┘
┌──────────────────────────────────────┐
│ deployment.md                 78%    │
│ Rate limiting is handled by…        │
└──────────────────────────────────────┘
```

Each source shows:
- **Document name** — the source file
- **Relevance score** — cosine similarity percentage
- **Text preview** — up to 3 lines of the matched chunk

---

## Admin View (`/admin`)

The admin panel manages collections and documents. On first access, it prompts for the API key.

### Authentication Gate

```
┌──────────────────────────────────┐
│  Admin Access                    │
│                                  │
│  Enter your API key to manage    │
│  collections and documents.      │
│                                  │
│  [Enter API key...]  [Set Key]  │
└──────────────────────────────────┘
```

The API key is persisted in `localStorage`. Use **Clear API Key** to sign out.

### Admin Panel Layout

```
┌───────────────────────┬──────────────────────────────┐
│  Admin Panel                     [Clear API Key]    │
├───────────────────────┼──────────────────────────────┤
│  COLLECTIONS   + New   │  DOCUMENTS           [📤 U…]│
│                       │                              │
│  ┌─────────────────┐  │  ┌────────────────────────┐ │
│  │ Technical Docs   │  │  │ 📄 spec.pdf            │ │
│  │ Project spec…    │  │  │ 200 KB · Jun 14, 2026  │ │
│  │ 5 documents     🗑️│  │                         🗑️│ │
│  └─────────────────┘  │  └────────────────────────┘ │
│  ┌─────────────────┐  │  ┌────────────────────────┐ │
│  │ API Reference    │  │  │ 📝 getting-started.md  │ │
│  │ OpenAPI specs    │  │  │ 12 KB · Jun 13, 2026   │ │
│  │ 2 documents     🗑️│  │                         🗑️│ │
│  └─────────────────┘  │  └────────────────────────┘ │
└───────────────────────┴──────────────────────────────┘
```

### Collection Manager

Located in the left panel of the admin view.

| Element | Description |
|---------|-------------|
| **Collection list** | Cards showing collection name, optional description, and document count |
| **Active collection** | Highlighted card with blue border. Documents shown correspond to this collection |
| **Create (+ New)** | Opens a modal dialog to create a new collection (name required, description optional) |
| **Delete (🗑️)** | Appears on hover. Removes the collection **and all its documents** after confirmation |

**Create Collection Dialog:**

```
┌──────────────────────────────────────┐
│  Create Collection                   │
├──────────────────────────────────────┤
│  Name                                │
│  [Technical Documentation        ]   │
│                                      │
│  Description (optional)              │
│  [Project specifications and     ]   │
│  [development guides             ]   │
│                                      │
├──────────────────────────────────────┤
│              [Cancel]  [Create]      │
└──────────────────────────────────────┘
```

### Document Management

Located in the right panel of the admin view.

| Element | Description |
|---------|-------------|
| **Upload button (📤 Upload)** | Opens the system file picker. Supports multiple file selection. Formats: `.pdf`, `.md`, `.txt`, `.html`, `.json`, `.zip` |
| **Upload progress** | Shows filename and percentage bar during upload |
| **Document list** | Each item shows file icon (by type), filename, file size, and upload date |
| **Delete (🗑️)** | Appears on hover. Removes the document after confirmation |

**Supported file types and their icons:**

| Format | Icon |
|--------|------|
| PDF | 📄 |
| Markdown | 📝 |
| HTML | 🌐 |
| JSON | 📋 |
| Plain text | 📃 |
| ZIP archive | 📦 |
| Other | 📎 |

**Upload flow:**

1. Select a collection first (documents are always added to the active collection)
2. Click **Upload** or drag files into the upload area
3. A progress bar shows the upload status for each file
4. On success, the document appears in the list with its size and upload timestamp
5. On failure, an error message is displayed inline

---

## Mobile Responsiveness

Both views adapt to narrow viewports (≤768 px):

- The session sidebar stacks **above** the chat window (max 200 px height)
- The admin panels stack vertically instead of side-by-side
- All other interactive elements remain functional without horizontal scrolling

## See Also

- [Architecture](architecture.md) — component structure and data flow
- [API Reference](api.md) — REST endpoints called by the frontend
- [Getting Started](getting-started.md) — installation and first run

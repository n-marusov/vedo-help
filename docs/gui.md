[← Architecture](architecture.md) · [Back to README](../README.md) · [API Reference →](api.md)

# User Interface Guide

The frontend is a single-page application (SPA) built with Vue 3 and TypeScript. It has two main views: **Chat** (Q&A interface) and **Admin** (collection and document management). The admin panel is accessible directly via `/admin`.

## Layout

The app layout is minimal — full-height `<router-view />` with no persistent nav sidebar, inspired by DeepSeek's clean design. A small "V" branding badge sits in the top-left corner (non-interactive).

---

## Chat View (`/`)

The chat view is the primary Q&A interface. It has a session sidebar on the left and the chat window on the right.

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
| **Delete (×)** | Appears on hover — removes the session after confirmation |

On mobile (<768px), the sidebar slides in from the left via a hamburger toggle. Selecting a session auto-closes the sidebar.

### Chat Window

```
┌───────────────────────────────────────────────┐
│                                               │
│              What would you like to know?      │
│     Ask questions about your documents and    │
│            get answers with citations.         │
│                                               │
│     Active collection: [Technical Docs ▼]      │
│                                               │
│  ┌────────────────────────────────────────┐   │
│  │ How do I configure the rate limiter?   │   │
│  └────────────────────────────────────────┘   │
│                                               │
│  ┌────────────────────────────────────────┐   │
│  │ The rate limiter is configured via…    │   │
│  │                                        │   │
│  │ ◀ 1 source                             │   │
│  └────────────────────────────────────────┘   │
│                                               │
├───────────────────────────────────────────────┤
│ ┌─────────────────────────────────────────┐   │
│ │ Ask a question...                    ➤  │   │
│ └─────────────────────────────────────────┘   │
└───────────────────────────────────────────────┘
```

| Element | Description |
|---------|-------------|
| **Welcome screen** | Clean centered layout with a "V" logo icon, tagline, and a collection selector dropdown (shown when no messages exist) |
| **Messages area** | Scrollable list of user and assistant messages. Centered layout with max-width for readability on wide screens |
| **Message bubbles** | User messages right-aligned with a subtle blue background. Assistant messages left-aligned with no background (clean, no-card look). Each uses `UserAvatar` instead of emoji |
| **Timestamp** | Subtle, small text below each message |
| **Avatar** | `UserAvatar` component: person silhouette for user, "V" icon for assistant. Inline SVG, zero network cost |
| **Sources section** | Collapsible panel under assistant messages. Uses a clean chevron icon. Shows document name and relevance score |
| **Input area** | Rounded input bar with inline send button. Textarea with auto-resize (supports Enter to send, Shift+Enter for newline). Send button lights up when text is entered |
| **Cancel (⏹ icon)** | Appears during streaming — aborts the current LLM response |
| **Error banner** | Red bar when the API returns an error |

### Streaming Flow

1. User types a question and presses Enter (or clicks the send icon)
2. A user message bubble appears immediately (optimistic update)
3. An empty assistant bubble appears with a **streaming glow bar** (animated gradient, replacing the old three-dot bounce)
4. As the backend streams tokens, the assistant bubble fills in progressively with a **blinking cursor** at the end
5. After completion, sources appear below the message with a chevron toggle
6. If the user clicks Cancel, streaming stops and the partial response is kept

### Message Sources

Click the **N sources** chevron to expand source citations:

```
▶ 2 sources
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

### Message Animations

New messages animate in with a smooth entrance effect:
- `opacity: 0 → 1` combined with `translateY(8px) → translateY(0)`
- Duration: 180ms (configurable via `--anim-msg-enter-duration`)
- Staggered: 50ms delay between consecutive messages (`--msg-index` CSS variable)
- Respects `prefers-reduced-motion` — animations disabled entirely

### Message Actions

Each message bubble reveals an action row on hover:

- **Edit (user messages only):** Pencil icon button, switches the message to a textarea with Save/Cancel buttons. Messages show a `· edited` badge after editing. The original content is preserved as an audit trail.
- **Delete (both roles):** Trash icon button, soft-deletes the message. Deleted messages are excluded from session history and exports.

### Loading Skeletons

Skeleton placeholders provide visual feedback during data loading:

- **Messages area:** Shown during `loadSession` via `VSkeleton variant="text" :rows="6"` while messages are being fetched
- **Sessions sidebar:** Shows card-style skeleton rows while `fetchSessions` is in progress
- **Document list (admin):** Shows card-style skeleton rows while documents are loading (`data-testid="documents-loading-skeleton"`)
- **Git repos list (admin):** Shows card-style skeleton rows while repos are loading (`data-testid="repos-loading-skeleton"`)

### Chat Export

The toolbar includes an **Export** button (ghost variant) with a format `<VSelect>` dropdown (Markdown / JSON). When clicked:
1. The session is fetched as a blob via `GET /api/sessions/:id/export?format={md|json}`
2. A download link is triggered programmatically
3. The file is downloaded as `session-{id}.md` or `session-{id}.json`
4. The button is disabled during export via `chatStore.isExporting`

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

## Design Tokens

The chat UI uses CSS custom properties defined in `frontend/src/assets/chat-tokens.css`. These tokens are the single source of truth for spacing, radii, animation timing, and message colors.

| Token | Default | Purpose |
|-------|---------|---------|
| `--msg-gap` | `0.75rem` | Gap between message and avatar |
| `--msg-padding-y` | `0.75rem` | Vertical padding in message content |
| `--msg-padding-x` | `1rem` | Horizontal padding in message content |
| `--msg-radius-user` | `18px 6px 18px 18px` | User bubble border radius |
| `--msg-radius-assistant` | `6px 18px 18px 18px` | Assistant content border radius |
| `--avatar-radius` | `999px` | Avatar circle radius |
| `--anim-msg-enter-duration` | `180ms` | Message entrance animation duration |
| `--anim-msg-enter-ease` | `cubic-bezier(0.2, 0, 0, 1)` | Message entrance easing |
| `--anim-stream-duration` | `1.4s` | Streaming glow animation duration |
| `--msg-user-bg` | `#1f5f9f` | User message background |
| `--msg-assistant-bg` | `#1e1e3a` | Assistant message background |
| `--msg-user-text` | `#ffffff` | User message text color |
| `--msg-assistant-text` | `#e8e8f2` | Assistant message text color |
| `--msg-time-color` | `#7d7da3` | Timestamp text color |
| `--avatar-user-bg` | `#2563eb` | User avatar background |
| `--avatar-assistant-bg` | `#2a2a4e` | Assistant avatar background |
| `--avatar-size` | `32px` | Avatar size (base) |
| `--max-msg-width` | `min(75%, 760px)` | Maximum message width |
| `--input-min-height` | `44px` | Input minimum height |

Tokens are logged to console at DEBUG level on app mount via `logChatTokenValues()`.

## Components

### UserAvatar

Located in `frontend/src/components/ui/UserAvatar.vue`.

**Props:**
- `role: 'user' | 'assistant'` — required, determines icon and colors
- `size?: 'sm' | 'md' | 'lg'` — optional, maps to `--avatar-size * factor` (0.75, 1.0, 1.25)

**Design:**
- User: solid circle with person silhouette SVG, `--avatar-user-bg` background
- Assistant: solid circle with "V" letter SVG, `--avatar-assistant-bg` background

### MessageBubble

Located in `frontend/src/components/MessageBubble.vue`.

**Props:**
- `message: Message` — the message data (role, content, sources, created_at)
- `isStreaming?: boolean` — enables streaming state animations
- `index?: number` — used for staggered entrance animation delay

**Features:**
- Markdown rendering via `marked` library (GFM enabled)
- **Syntax highlighting** — code blocks rendered via `highlight.js` with dark theme, supports Python, Rust, TypeScript, JavaScript, Bash, JSON, SQL, CSS, HTML, Markdown, and plaintext
- **Language labels** — each code block shows the detected or explicit language name in the header (`code-lang-label`)
- **Copy button** — each code block has a "Copy" button that copies the code content to clipboard; shows "Copied!" state for 2 seconds
- **GFM tables** — tables render with `<thead>`, `<tbody>`, alternating row colors, and border styling
- **Blockquotes** — styled with left primary border and muted background
- **Lists** — ordered and unordered with proper indentation and markers
- **Headings** — `h1`–`h6` proportional sizing via design tokens
- **Horizontal rules** — styled via design tokens
- **Images** — max-width constrained, border-radius
- Streaming glow bar (when `isStreaming && !message.content`)
- Blinking cursor (when `isStreaming && message.content`)
- Collapsible sources section
- Smooth entrance animation with staggered delay
- **Edit mode (user only):** hover to reveal edit/delete buttons; textarea with Save/Cancel (`· edited` badge on edited messages)
- **Delete mode (both roles):** hover to reveal delete button; soft-deletes the message

**Markdown module:** `frontend/src/utils/markdown.ts` — custom `marked` renderer with `highlight.js` integration, imports only used languages for bundle size optimization.

### ChatWindow

Located in `frontend/src/components/ChatWindow.vue`.

**Features:**
- Welcome screen with collection selector
- Max-width centered layout (820px)
- Auto-resizing textarea input
- Inline send/cancel buttons
- Scrollbar styling

---

## Mobile Responsiveness

The chat layout adapts to all viewports:

- **< 480px (mobile):** Full-screen chat, sidebar slides in as an overlay, input at bottom with safe-area padding (`env(safe-area-inset-bottom)`), messages use full width
- **480px – 768px (tablet):** Collapsible sidebar via hamburger toggle, message max-width capped at 95%
- **> 768px (desktop):** Split layout with persistent sidebar, messages centered at 820px max-width

Admin view adapts similarly — panels stack vertically at narrow widths.

## See Also

- [Architecture](architecture.md) — component structure and data flow
- [API Reference](api.md) — REST endpoints called by the frontend
- [Getting Started](getting-started.md) — installation and first run

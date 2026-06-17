<script setup lang="ts">
import type { Message, SourceRef } from "@/api/types";
import UserAvatar from "@/components/ui/UserAvatar.vue";
import { decodeCode, renderMarkdown } from "@/utils/markdown";
import { computed, onMounted, ref, watch } from "vue";

const props = defineProps<{
  message: Message;
  isStreaming?: boolean;
  index?: number;
}>();

const sourcesExpanded = ref(false);

const renderedContent = computed(() => {
  if (!props.message.content) return "";
  return renderMarkdown(props.message.content);
});

const parsedSources = computed<SourceRef[]>(() => {
  if (!props.message.sources) return [];
  try {
    return JSON.parse(props.message.sources) as SourceRef[];
  } catch {
    return [];
  }
});

function toggleSources() {
  sourcesExpanded.value = !sourcesExpanded.value;
}

function handleMarkdownClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  const btn = target.closest(".copy-code-btn") as HTMLElement | null;
  if (!btn || !btn.dataset.code) return;

  const code = decodeCode(btn.dataset.code);
  navigator.clipboard
    .writeText(code)
    .then(() => {
      const originalText = btn.textContent;
      btn.textContent = "Copied!";
      console.info("[MessageBubble] code copied", { length: code.length });
      setTimeout(() => {
        btn.textContent = originalText;
      }, 2000);
    })
    .catch((err) => {
      console.warn("[MessageBubble] copy failed", err);
    });
}
const formattedTime = computed(() => {
  return new Date(props.message.created_at).toLocaleTimeString([], {
    hour: "2-digit",
    minute: "2-digit",
  });
});

onMounted(() => {
  console.debug("[MessageBubble] mounted", {
    role: props.message.role,
    contentLength: props.message.content?.length || 0,
    sourcesCount: parsedSources.value.length,
  });

  if (props.isStreaming) {
    console.debug("[MessageBubble] streaming started", {
      messageId: props.message.id,
    });
  }
});

watch(
  () => props.isStreaming,
  (streaming, wasStreaming) => {
    if (streaming && !wasStreaming) {
      console.debug("[MessageBubble] streaming started", {
        messageId: props.message.id,
      });
    } else if (!streaming && wasStreaming) {
      console.debug("[MessageBubble] streaming ended", {
        messageId: props.message.id,
      });
    }
  },
);
</script>

<template>
  <div
    class="message-bubble"
    :class="[
      message.role === 'user' ? 'message-user' : 'message-assistant',
      'message-enter',
    ]"
    :style="{ '--msg-index': index ?? 0 }"
    :data-testid="
      message.role === 'user' ? 'message-user' : 'message-assistant'
    "
  >
    <!-- Avatar (assistant on left, user on right) -->
    <UserAvatar
      v-if="message.role === 'assistant'"
      :role="message.role"
      size="sm"
      class="message-avatar"
    />

    <div class="message-content-wrapper">
      <!-- Message content -->
      <div
        class="message-content"
        :class="{
          'message-content--user': message.role === 'user',
          'message-content--assistant': message.role === 'assistant',
        }"
        :data-testid="'message-body-' + message.role"
      >
        <div
          v-if="message.content"
          class="markdown-body"
          data-testid="message-content"
          v-html="renderedContent"
          @click.stop="handleMarkdownClick"
        />
        <span
          v-if="isStreaming && message.content"
          class="streaming-cursor"
          aria-hidden="true"
        />
        <div v-if="isStreaming && !message.content" class="streaming-bar" />
      </div>

      <!-- Timestamp -->
      <div class="message-meta">
        <span class="message-time" data-testid="message-time">{{
          formattedTime
        }}</span>
      </div>

      <!-- Sources -->
      <div
        v-if="parsedSources.length > 0 && message.role === 'assistant'"
        class="sources-section"
      >
        <button
          class="sources-toggle"
          data-testid="sources-toggle"
          @click="toggleSources"
        >
          <svg
            aria-hidden="true"
            class="sources-chevron"
            :class="{ expanded: sourcesExpanded }"
            fill="none"
            height="12"
            viewBox="0 0 12 12"
            width="12"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              d="M4 2.5L7.5 6L4 9.5"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="1.5"
            />
          </svg>
          <span
            >{{ parsedSources.length }} source{{
              parsedSources.length > 1 ? "s" : ""
            }}</span
          >
        </button>
        <div
          v-if="sourcesExpanded"
          class="sources-list"
          data-testid="sources-list"
        >
          <div
            v-for="(source, idx) in parsedSources"
            :key="idx"
            class="source-item"
            data-testid="source-item"
          >
            <div class="source-header">
              <span class="source-doc" data-testid="source-document">{{
                source.document_name
              }}</span>
              <span class="source-relevance" data-testid="source-relevance">
                {{ Math.round(source.relevance * 100) }}%
              </span>
            </div>
            <p class="source-text">{{ source.text }}</p>
          </div>
        </div>
      </div>
    </div>

    <!-- User avatar on the right -->
    <UserAvatar
      v-if="message.role === 'user'"
      :role="message.role"
      size="sm"
      class="message-avatar"
    />
  </div>
</template>

<style scoped>
.message-bubble {
  display: flex;
  gap: var(--msg-gap);
  padding: 0.375rem 1.5rem;
  max-width: var(--max-msg-width);
}

.message-user {
  align-self: flex-end;
  flex-direction: row-reverse;
}

.message-assistant {
  align-self: flex-start;
}

/* ===== Avatar ===== */
.message-avatar {
  flex-shrink: 0;
  margin-top: 0.15rem;
}

/* ===== Content wrapper ===== */
.message-content-wrapper {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.message-assistant .message-content-wrapper {
  align-items: flex-start;
}

.message-user .message-content-wrapper {
  align-items: flex-end;
}

/* ===== Message content ===== */
.message-content {
  font-size: 0.9rem;
  line-height: 1.6;
  word-break: break-word;
  padding: var(--msg-padding-y) var(--msg-padding-x);
}

.message-content--user {
  background: var(--msg-user-bg);
  color: var(--msg-user-text);
  border-radius: var(--msg-radius-user);
}

.message-content--assistant {
  background: transparent;
  color: var(--msg-assistant-text);
  border-radius: var(--msg-radius-assistant);
  padding-left: 0;
}

.markdown-body :deep(p) {
  margin: 0.35rem 0;
}

.markdown-body :deep(p:first-child) {
  margin-top: 0;
}

.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}

.markdown-body :deep(code) {
  background: var(--color-border);
  padding: 0.15rem 0.35rem;
  border-radius: var(--radius-xs);
  font-size: 0.85em;
  font-family: var(--font-family);
  color: var(--color-foreground);
}

.markdown-body :deep(a) {
  color: #6b9fff;
}

/* ===== GFM Tables ===== */
.markdown-body :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 0.5rem 0;
  font-size: 0.85rem;
}

.markdown-body :deep(th) {
  background: var(--color-secondary);
  color: var(--color-foreground);
  font-weight: 600;
  padding: 0.5rem 0.75rem;
  text-align: left;
  border: 1px solid var(--color-border);
}

.markdown-body :deep(td) {
  padding: 0.4rem 0.75rem;
  border: 1px solid var(--color-border);
}

.markdown-body :deep(tr:nth-child(even)) {
  background: var(--color-secondary);
}

/* ===== Blockquotes ===== */
.markdown-body :deep(blockquote) {
  margin: 0.5rem 0;
  padding: 0.25rem 0.75rem;
  border-left: 3px solid var(--color-primary);
  color: var(--color-muted-foreground);
  background: var(--color-secondary);
  border-radius: 0 var(--radius-xs) var(--radius-xs) 0;
}

.markdown-body :deep(blockquote p) {
  margin: 0.25rem 0;
}

/* ===== Lists ===== */
.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  margin: 0.35rem 0;
  padding-left: 1.5rem;
}

.markdown-body :deep(li) {
  margin: 0.15rem 0;
}

.markdown-body :deep(ul > li) {
  list-style-type: disc;
}

.markdown-body :deep(ol > li) {
  list-style-type: decimal;
}

/* ===== Horizontal Rules ===== */
.markdown-body :deep(hr) {
  border: none;
  height: 1px;
  background: var(--color-border);
  margin: 1rem 0;
}

/* ===== Headings ===== */
.markdown-body :deep(h1) {
  font-size: 1.5rem;
  font-weight: 700;
  margin: 1rem 0 0.5rem;
  color: var(--color-foreground);
}

.markdown-body :deep(h2) {
  font-size: 1.3rem;
  font-weight: 700;
  margin: 0.85rem 0 0.4rem;
  color: var(--color-foreground);
}

.markdown-body :deep(h3) {
  font-size: 1.1rem;
  font-weight: 600;
  margin: 0.7rem 0 0.35rem;
  color: var(--color-foreground);
}

.markdown-body :deep(h4) {
  font-size: 1rem;
  font-weight: 600;
  margin: 0.6rem 0 0.3rem;
  color: var(--color-foreground);
}

.markdown-body :deep(h5),
.markdown-body :deep(h6) {
  font-size: 0.9rem;
  font-weight: 600;
  margin: 0.5rem 0 0.25rem;
  color: var(--color-muted-foreground);
}

/* ===== Images ===== */
.markdown-body :deep(img) {
  max-width: 100%;
  border-radius: var(--radius-sm);
  margin: 0.5rem 0;
}

/* ===== Code blocks with syntax highlighting ===== */
.markdown-body :deep(.code-block-wrapper) {
  margin: 0.5rem 0;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  overflow: hidden;
  background: var(--color-card);
}

.markdown-body :deep(.code-block-header) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.35rem 0.75rem;
  background: var(--color-secondary);
  border-bottom: 1px solid var(--color-border);
}

.markdown-body :deep(.code-lang-label) {
  font-size: 0.7rem;
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  line-height: 1;
}

.markdown-body :deep(.copy-code-btn) {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  padding: 0.2rem 0.5rem;
  font-size: 0.7rem;
  font-family: var(--font-family);
  color: var(--color-muted-foreground);
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xs);
  cursor: pointer;
  transition:
    color var(--transition-fast),
    border-color var(--transition-fast);
  line-height: 1.4;
}

.markdown-body :deep(.copy-code-btn:hover) {
  color: var(--color-foreground);
  border-color: var(--color-input);
}

.markdown-body :deep(.copy-code-btn:active) {
  opacity: 0.8;
}

.markdown-body :deep(pre) {
  margin: 0;
  padding: 0.75rem;
  overflow-x: auto;
  background: var(--color-card);
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
  font-size: 0.82rem;
  line-height: 1.5;
}

/* highlight.js dark theme overrides */
.markdown-body :deep(.hljs) {
  color: var(--color-foreground);
  background: transparent;
}

.markdown-body :deep(.hljs-keyword) {
  color: #c792ea;
}

.markdown-body :deep(.hljs-string) {
  color: #c3e88d;
}

.markdown-body :deep(.hljs-number) {
  color: #f78c6c;
}

.markdown-body :deep(.hljs-comment) {
  color: #676e95;
  font-style: italic;
}

.markdown-body :deep(.hljs-function) {
  color: #82aaff;
}

.markdown-body :deep(.hljs-title) {
  color: #82aaff;
}

.markdown-body :deep(.hljs-built_in) {
  color: #ffcb6b;
}

.markdown-body :deep(.hljs-type) {
  color: #ffcb6b;
}

.markdown-body :deep(.hljs-literal) {
  color: #f78c6c;
}

.markdown-body :deep(.hljs-attr) {
  color: #f07178;
}

.markdown-body :deep(.hljs-attribute) {
  color: #c792ea;
}

.markdown-body :deep(.hljs-selector-tag),
.markdown-body :deep(.hljs-selector-class),
.markdown-body :deep(.hljs-selector-id) {
  color: #ffcb6b;
}

.markdown-body :deep(.hljs-meta) {
  color: #89ddff;
}

.markdown-body :deep(.hljs-tag) {
  color: #f07178;
}

.markdown-body :deep(.hljs-name) {
  color: #f07178;
}

.markdown-body :deep(.hljs-variable) {
  color: #f07178;
}

.markdown-body :deep(.hljs-params) {
  color: var(--color-foreground);
}

.markdown-body :deep(.hljs-symbol) {
  color: #c792ea;
}

.markdown-body :deep(.hljs-section) {
  color: #82aaff;
}

.markdown-body :deep(.hljs-addition) {
  color: #c3e88d;
}

.markdown-body :deep(.hljs-deletion) {
  color: #f07178;
}

.markdown-body :deep(.hljs-emphasis) {
  font-style: italic;
}

.markdown-body :deep(.hljs-strong) {
  font-weight: bold;
}

/* ===== Streaming states ===== */
.streaming-cursor {
  display: inline-block;
  width: 2px;
  height: 1em;
  background: var(--msg-assistant-text);
  margin-left: 1px;
  vertical-align: text-bottom;
  animation: streamingBlink 0.8s step-end infinite;
}

@keyframes streamingBlink {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0;
  }
}

.streaming-bar {
  height: 4px;
  width: 40px;
  background: linear-gradient(
    90deg,
    transparent,
    var(--msg-assistant-text),
    transparent
  );
  border-radius: 2px;
  animation: streamingGlow var(--anim-stream-duration) ease-in-out infinite;
}

@keyframes streamingGlow {
  0% {
    opacity: 0.2;
    transform: translateX(-8px);
  }
  50% {
    opacity: 1;
    transform: translateX(0);
  }
  100% {
    opacity: 0.2;
    transform: translateX(8px);
  }
}

/* ===== Message meta (timestamp) ===== */
.message-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.15rem var(--msg-padding-x);
}

.message-user .message-meta {
  justify-content: flex-end;
}

.message-time {
  font-size: 0.65rem;
  color: var(--msg-time-color);
  line-height: 1;
}

/* ===== Sources ===== */
.sources-section {
  margin-top: 0.25rem;
  padding: 0 var(--msg-padding-x);
}

.sources-toggle {
  display: inline-flex;
  align-items: center;
  gap: 0.3rem;
  background: none;
  border: none;
  padding: 0.2rem 0;
  color: var(--msg-time-color);
  font-size: 0.75rem;
  cursor: pointer;
  transition: color 0.15s;
}

.sources-toggle:hover {
  color: var(--msg-assistant-text);
}

.sources-chevron {
  transition: transform 0.2s;
  flex-shrink: 0;
}

.sources-chevron.expanded {
  transform: rotate(90deg);
}

.sources-list {
  margin-top: 0.4rem;
  display: flex;
  flex-direction: column;
  gap: 0.4rem;
}

.source-item {
  background: #1a1a32;
  border-radius: 6px;
  padding: 0.45rem 0.65rem;
  border-left: 3px solid var(--avatar-user-bg);
}

.source-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.2rem;
}

.source-doc {
  font-size: 0.72rem;
  font-weight: 600;
  color: #7d7da3;
}

.source-relevance {
  font-size: 0.68rem;
  color: #4caf50;
  font-weight: 600;
}

.source-text {
  font-size: 0.75rem;
  color: #9a9abc;
  margin: 0;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* ===== Role label ===== */
.message-role-label {
  display: block;
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--msg-time-color);
  margin-bottom: 0.15rem;
  padding: 0 var(--msg-padding-x);
  line-height: 1;
}

.message-user .message-role-label {
  text-align: right;
}

.message-assistant .message-role-label {
  text-align: left;
}

/* ===== Entrance animation ===== */
.message-enter {
  animation: messageEnter var(--anim-msg-enter-duration)
    var(--anim-msg-enter-ease) both;
  animation-delay: calc(var(--msg-index, 0) * 50ms);
}

@keyframes messageEnter {
  from {
    opacity: 0;
    transform: translateY(8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

/* Respect reduced motion */
@media (prefers-reduced-motion: reduce) {
  .message-enter {
    animation: none;
  }
}
</style>

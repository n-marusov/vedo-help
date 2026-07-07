<script setup lang="ts">
import type { Message } from '@/api/types';
import { decodeCode, renderMarkdown } from '@/utils/markdown';
import { computed, onMounted, ref } from 'vue';

const props = defineProps<{
  message: Message;
  isStreaming?: boolean;
  index?: number;
  pipelineStageLabel?: string | null;
}>();

const emit = defineEmits<{
  edit: [{ id: string }];
  'save-edit': [{ id: string; content: string }];
  'cancel-edit': [];
  copy: [{ id: string }];
  regenerate: [{ id: string }];
}>();

const editing = ref(false);
const draftContent = ref('');
const copyFeedback = ref(false);

const renderedContent = computed(() => {
  if (!props.message.content) return '';
  return renderMarkdown(props.message.content);
});

function handleMarkdownClick(event: MouseEvent) {
  const target = event.target as HTMLElement;
  const btn = target.closest('.copy-code-btn') as HTMLElement | null;
  if (!btn || !btn.dataset.code) return;

  const code = decodeCode(btn.dataset.code);
  navigator.clipboard
    .writeText(code)
    .then(() => {
      const originalText = btn.textContent;
      btn.textContent = 'Copied!';
      setTimeout(() => {
        btn.textContent = originalText;
      }, 2000);
    })
    .catch((err) => {
      console.warn('[MessageBubble] copy failed', err);
    });
}

const formattedTime = computed(() => {
  return new Date(props.message.created_at).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  });
});

const isPersistedMessage = computed(() => {
  return !props.message.id.startsWith('temp-');
});

function startEdit() {
  if (!isPersistedMessage.value) {
    return;
  }
  draftContent.value = props.message.content;
  editing.value = true;
  emit('edit', { id: props.message.id });
}

function saveEdit() {
  emit('save-edit', { id: props.message.id, content: draftContent.value });
  editing.value = false;
}

function cancelEdit() {
  editing.value = false;
  draftContent.value = props.message.content;
}

async function handleCopy() {
  emit('copy', { id: props.message.id });
  copyFeedback.value = true;
  setTimeout(() => {
    copyFeedback.value = false;
  }, 1500);
}

function handleRegenerate() {
  emit('regenerate', { id: props.message.id });
}

onMounted(() => {
  // no-op: streaming state handled by the template
});
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
    <div class="message-content-wrapper">
      <!-- Role label -->
      <div class="message-role-label">
        {{ message.role === "user" ? "You" : "Assistant" }}
      </div>

      <!-- Message content -->
      <div
        class="message-content"
        :class="{
          'message-content--user': message.role === 'user',
          'message-content--assistant': message.role === 'assistant',
        }"
        :data-testid="'message-body-' + message.role"
      >
        <!-- Editing mode: textarea + Save/Cancel -->
        <template v-if="editing">
          <textarea
            v-model="draftContent"
            class="message-edit-textarea"
            data-testid="message-edit-textarea"
          />
          <div class="message-edit-actions">
            <button
              class="message-edit-save"
              data-testid="message-save-btn"
              @click="saveEdit"
            >
              Save
            </button>
            <button
              class="message-edit-cancel"
              data-testid="message-cancel-edit-btn"
              @click="cancelEdit"
            >
              Cancel
            </button>
          </div>
        </template>

        <!-- Normal display mode -->
        <template v-else>
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
          <div
            v-if="isStreaming && !message.content"
            class="streaming-bar"
            data-testid="streaming-indicator"
          >
            <template v-if="pipelineStageLabel">
              <span class="streaming-stage-label">{{
                pipelineStageLabel
              }}</span>
              <span class="streaming-dots">...</span>
            </template>
            <template v-else>
              Assistant is typing<span class="streaming-dots">...</span>
            </template>
          </div>
        </template>
      </div>

      <!-- Actions + Timestamp row -->
      <div class="message-footer">
        <div class="message-actions" data-testid="message-actions-row">
          <!-- Timestamp -->
          <span class="message-time" data-testid="message-time">{{
            formattedTime
          }}</span>

          <!-- Copy button (both roles) -->
          <button
            v-if="isPersistedMessage"
            class="message-action-btn"
            data-testid="message-copy-btn"
            :title="copyFeedback ? 'Copied!' : 'Copy'"
            @click="handleCopy"
          >
            <svg
              v-if="!copyFeedback"
              fill="none"
              height="14"
              viewBox="0 0 14 14"
              width="14"
              xmlns="http://www.w3.org/2000/svg"
            >
              <rect
                x="3.5"
                y="3.5"
                width="9"
                height="9"
                rx="1"
                stroke="currentColor"
                stroke-width="1.2"
              />
              <path
                d="M10.5 3.5V2.5C10.5 1.5 9.8 1 9 1H3C2.2 1 1.5 1.5 1.5 2.5V9.5C1.5 10.5 2.2 11 3 11H4"
                stroke="currentColor"
                stroke-width="1.2"
              />
            </svg>
            <svg
              v-else
              fill="none"
              height="14"
              viewBox="0 0 14 14"
              width="14"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M3 7.5L5.5 10L11 4"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
            </svg>
          </button>

          <!-- Edit button (user messages only) -->
          <button
            v-if="message.role === 'user' && isPersistedMessage"
            class="message-action-btn"
            data-testid="message-edit-btn"
            title="Edit"
            @click="startEdit"
          >
            <svg
              fill="none"
              height="14"
              viewBox="0 0 14 14"
              width="14"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M10 1.5L12.5 4L5 11.5L1.5 12.5L2.5 9L10 1.5Z"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1.2"
              />
            </svg>
          </button>

          <!-- Regenerate button (assistant messages only) -->
          <button
            v-if="message.role === 'assistant' && isPersistedMessage"
            class="message-action-btn"
            data-testid="message-regenerate-btn"
            title="Regenerate"
            @click="handleRegenerate"
          >
            <svg
              fill="none"
              height="14"
              viewBox="0 0 14 14"
              width="14"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M2 7C2 4.2 4.2 2 7 2C9.8 2 11 4.5 11 4.5"
                stroke="currentColor"
                stroke-width="1.2"
                stroke-linecap="round"
              />
              <path
                d="M11 4.5H9"
                stroke="currentColor"
                stroke-width="1.2"
                stroke-linecap="round"
              />
              <path
                d="M12 7C12 9.8 9.8 12 7 12C4.2 12 3 9.5 3 9.5"
                stroke="currentColor"
                stroke-width="1.2"
                stroke-linecap="round"
              />
              <path
                d="M3 9.5H5"
                stroke="currentColor"
                stroke-width="1.2"
                stroke-linecap="round"
              />
            </svg>
          </button>

          <!-- Edited badge -->
          <span
            v-if="message.edited_at"
            class="message-edited-badge"
            data-testid="message-edited-badge"
          >
            · edited
          </span>
        </div>
      </div>
    </div>
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
}

.message-assistant {
  align-self: flex-start;
}

.message-avatar {
  flex-shrink: 0;
  width: var(--avatar-size);
  height: var(--avatar-size);
  border-radius: var(--avatar-radius);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.75rem;
  font-weight: 600;
  margin-top: 0.25rem;
}

.message-content-wrapper {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  min-width: 0;
  flex: 0 1 auto;
}

.message-assistant .message-content-wrapper {
  align-items: flex-start;
}

.message-user .message-content-wrapper {
  align-items: flex-end;
}

.message-role-label {
  font-size: 0.7rem;
  font-weight: 600;
  color: var(--msg-time-color);
  padding: 0 0.25rem;
  margin-bottom: 0.125rem;
}

.message-content {
  padding: var(--msg-padding-y) var(--msg-padding-x);
  line-height: 1.55;
  font-size: 0.875rem;
  position: relative;
  word-wrap: break-word;
  overflow-wrap: break-word;
}

.message-content--user {
  background: var(--msg-user-bg);
  color: var(--msg-user-text);
  border-radius: var(--msg-radius-user);
  max-width: 520px;
}

.message-content--assistant {
  background: var(--msg-assistant-bg);
  color: var(--msg-assistant-text);
  border-radius: var(--msg-radius-assistant);
  max-width: 600px;
}

/* Inline code styling */
.markdown-body :deep(p) {
  margin: 0.25rem 0;
}

.markdown-body :deep(p:first-child) {
  margin-top: 0;
}

.markdown-body :deep(p:last-child) {
  margin-bottom: 0;
}

.markdown-body :deep(code) {
  background: rgba(128, 128, 128, 0.15);
  padding: 0.125rem 0.375rem;
  border-radius: 4px;
  font-size: 0.8em;
  font-family: "IBM Plex Mono", "SF Mono", "Fira Code", monospace;
}

.markdown-body :deep(a) {
  color: var(--color-primary);
  text-decoration: underline;
}

.markdown-body :deep(table) {
  border-collapse: collapse;
  width: 100%;
  margin: 0.5rem 0;
  font-size: 0.8125rem;
}

.markdown-body :deep(th) {
  background: rgba(128, 128, 128, 0.1);
  font-weight: 600;
  text-align: left;
  padding: 0.375rem 0.5rem;
  border: 1px solid rgba(128, 128, 128, 0.2);
}

.markdown-body :deep(td) {
  padding: 0.375rem 0.5rem;
  border: 1px solid rgba(128, 128, 128, 0.2);
}

/* No alternating row colors — all rows same background */
.markdown-body :deep(tr:nth-child(even)) {
  background: transparent;
}

.markdown-body :deep(hr) {
  border: none;
  border-top: 1px solid var(--color-border, rgba(128, 128, 128, 0.3));
  margin: 1rem 0;
}

.markdown-body :deep(blockquote) {
  border-left: 3px solid var(--color-primary);
  margin: 0.5rem 0;
  padding: 0.25rem 0.75rem;
  color: var(--color-muted-foreground);
}

.markdown-body :deep(blockquote p) {
  margin: 0;
}

.markdown-body :deep(ul),
.markdown-body :deep(ol) {
  padding-left: 1.25rem;
  margin: 0.25rem 0;
}

.markdown-body :deep(li) {
  margin: 0.125rem 0;
}

.markdown-body :deep(ul > li) {
  list-style-type: disc;
}

.markdown-body :deep(ol > li) {
  list-style-type: decimal;
}

.markdown-body :deep(hr) {
  border: none;
  border-top: 1px solid rgba(128, 128, 128, 0.2);
  margin: 0.75rem 0;
}

.markdown-body :deep(h1) {
  font-size: 1.25rem;
  font-weight: 700;
  margin: 0.75rem 0 0.5rem;
  padding-bottom: 0.25rem;
  border-bottom: 1px solid rgba(128, 128, 128, 0.15);
}

.markdown-body :deep(h2) {
  font-size: 1.1rem;
  font-weight: 700;
  margin: 0.75rem 0 0.375rem;
}

.markdown-body :deep(h3) {
  font-size: 1rem;
  font-weight: 600;
  margin: 0.5rem 0 0.25rem;
}

.markdown-body :deep(h4) {
  font-size: 0.9375rem;
  font-weight: 600;
  margin: 0.5rem 0 0.25rem;
}

.markdown-body :deep(h5),
.markdown-body :deep(h6) {
  font-size: 0.875rem;
  font-weight: 600;
  margin: 0.375rem 0 0.25rem;
}

.markdown-body :deep(img) {
  max-width: 100%;
  height: auto;
  border-radius: 8px;
  margin: 0.5rem 0;
}

/* Code block wrapper */
.markdown-body :deep(.code-block-wrapper) {
  position: relative;
  margin: 0.5rem 0;
  border-radius: 8px;
  overflow: hidden;
  background: rgba(0, 0, 0, 0.05);
}

[data-theme="dark"] .markdown-body :deep(.code-block-wrapper) {
  background: rgba(255, 255, 255, 0.05);
}

.markdown-body :deep(.code-block-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 0.375rem 0.75rem;
  font-size: 0.75rem;
  color: var(--color-muted-foreground);
  border-bottom: 1px solid rgba(128, 128, 128, 0.15);
}

.markdown-body :deep(.code-lang-label) {
  font-family: "IBM Plex Mono", "SF Mono", "Fira Code", monospace;
  text-transform: uppercase;
  font-size: 0.65rem;
  letter-spacing: 0.05em;
}

.markdown-body :deep(.copy-code-btn) {
  background: transparent;
  border: 1px solid rgba(128, 128, 128, 0.25);
  color: var(--color-muted-foreground);
  font-size: 0.7rem;
  padding: 0.125rem 0.5rem;
  border-radius: 4px;
  cursor: pointer;
  transition: all 0.15s;
}

.markdown-body :deep(.copy-code-btn:hover) {
  background: rgba(128, 128, 128, 0.15);
  color: var(--color-foreground);
}

.markdown-body :deep(.copy-code-btn:active) {
  transform: scale(0.97);
}

.markdown-body :deep(pre) {
  padding: 0.75rem 1rem;
  overflow-x: auto;
  margin: 0;
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
  border-radius: 0;
  font-size: 0.8125rem;
  line-height: 1.5;
}

.markdown-body :deep(.hljs) {
  display: block;
  overflow-x: auto;
}

/* Syntax highlighting tokens */
.markdown-body :deep(.hljs-keyword) {
  color: #7c3aed;
}
.markdown-body :deep(.hljs-string) {
  color: #059669;
}
.markdown-body :deep(.hljs-number) {
  color: #d97706;
}
.markdown-body :deep(.hljs-comment) {
  color: #6b7280;
  font-style: italic;
}
.markdown-body :deep(.hljs-function) {
  color: #2563eb;
}
.markdown-body :deep(.hljs-title) {
  color: #2563eb;
}
.markdown-body :deep(.hljs-built_in) {
  color: #dc2626;
}
.markdown-body :deep(.hljs-type) {
  color: #059669;
}
.markdown-body :deep(.hljs-literal) {
  color: #7c3aed;
}
.markdown-body :deep(.hljs-attr) {
  color: #d97706;
}
.markdown-body :deep(.hljs-attribute) {
  color: #d97706;
}
.markdown-body :deep(.hljs-selector-tag),
.markdown-body :deep(.hljs-selector-class),
.markdown-body :deep(.hljs-selector-id) {
  color: #dc2626;
}
.markdown-body :deep(.hljs-meta) {
  color: #6b7280;
}
.markdown-body :deep(.hljs-tag) {
  color: #2563eb;
}
.markdown-body :deep(.hljs-name) {
  color: #dc2626;
}
.markdown-body :deep(.hljs-variable) {
  color: #d97706;
}
.markdown-body :deep(.hljs-params) {
  color: #d97706;
}
.markdown-body :deep(.hljs-symbol) {
  color: #7c3aed;
}
.markdown-body :deep(.hljs-section) {
  color: #2563eb;
  font-weight: 700;
}
.markdown-body :deep(.hljs-addition) {
  color: #059669;
}
.markdown-body :deep(.hljs-deletion) {
  color: #dc2626;
}
.markdown-body :deep(.hljs-emphasis) {
  font-style: italic;
}
.markdown-body :deep(.hljs-strong) {
  font-weight: 700;
}

/* Streaming indicators */
.streaming-cursor {
  display: inline-block;
  width: 0.5rem;
  height: 1rem;
  background: currentColor;
  animation: blink 0.8s step-end infinite;
  margin-left: 0.125rem;
  vertical-align: text-bottom;
}

@keyframes blink {
  50% {
    opacity: 0;
  }
}

.streaming-bar {
  color: var(--color-muted-foreground);
  font-family: "IBM Plex Mono", monospace;
  font-size: 13px;
  font-weight: 600;
  padding: 8px 0;
}

.streaming-dots {
  animation: blink 1.4s steps(4) infinite;
}

.streaming-stage-label {
  color: var(--color-muted-foreground);
  font-family: "IBM Plex Mono", monospace;
  font-size: 13px;
  font-weight: 600;
}

@keyframes blink {
  0%,
  100% {
    opacity: 0;
  }
  50% {
    opacity: 1;
  }
}

/* Action buttons row */
.message-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  padding: 0 0.25rem;
  min-height: 24px;
}

.message-actions {
  display: flex;
  align-items: center;
  gap: 0.25rem;
  opacity: 0;
  transition: opacity 0.15s;
}

.message-bubble:hover .message-actions {
  opacity: 1;
}

.message-action-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border-radius: 6px;
  border: none;
  background: transparent;
  color: var(--msg-time-color);
  cursor: pointer;
  transition: all 0.12s;
}

.message-action-btn:hover {
  background: rgba(128, 128, 128, 0.12);
  color: var(--color-foreground);
}

/* Edit textarea */
.message-edit-textarea {
  width: 100%;
  min-height: 60px;
  padding: 0.5rem;
  border: 1px solid rgba(128, 128, 128, 0.3);
  border-radius: 8px;
  font-family: inherit;
  font-size: 0.875rem;
  line-height: 1.5;
  resize: vertical;
  background: var(--color-input);
  color: var(--color-foreground);
  outline: none;
}

.message-edit-textarea:focus {
  border-color: var(--color-primary);
  box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}

.message-edit-actions {
  display: flex;
  gap: 0.375rem;
  margin-top: 0.375rem;
}

.message-edit-save {
  padding: 0.25rem 0.75rem;
  border-radius: 6px;
  border: none;
  background: var(--color-primary);
  color: white;
  font-size: 0.8rem;
  cursor: pointer;
}

.message-edit-save:hover {
  opacity: 0.9;
}

.message-edit-cancel {
  padding: 0.25rem 0.75rem;
  border-radius: 6px;
  border: 1px solid rgba(128, 128, 128, 0.3);
  background: transparent;
  color: var(--color-foreground);
  font-size: 0.8rem;
  cursor: pointer;
}

.message-edit-cancel:hover {
  background: rgba(128, 128, 128, 0.1);
}

.message-time {
  font-size: 0.65rem;
  color: var(--msg-time-color);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.message-edited-badge {
  font-style: italic;
  opacity: 0.7;
}

/* Role label removed from individual messages, shown as text above content */
.message-role-label {
  display: none;
}

/* Entry animation */
.message-enter {
  animation: message-enter var(--anim-msg-enter-duration)
    var(--anim-msg-enter-ease) both;
  animation-delay: calc(var(--msg-index, 0) * 30ms);
}

@keyframes message-enter {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

@media (prefers-reduced-motion: reduce) {
  .message-enter {
    animation: none;
  }
}
</style>

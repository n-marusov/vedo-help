<script setup lang="ts">
import type { Message, SourceRef } from '@/api/types';
import UserAvatar from '@/components/ui/UserAvatar.vue';
import { marked } from 'marked';
import { computed, onMounted, ref, watch } from 'vue';

const props = defineProps<{
  message: Message;
  isStreaming?: boolean;
  index?: number;
}>();

const sourcesExpanded = ref(false);

const renderedContent = computed(() => {
  if (!props.message.content) return '';
  try {
    return marked.parse(props.message.content, { async: false }) as string;
  } catch (err) {
    console.warn('[MessageBubble] marked.parse failed', err);
    return props.message.content;
  }
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

const formattedTime = computed(() => {
  return new Date(props.message.created_at).toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
  });
});

onMounted(() => {
  console.debug('[MessageBubble] mounted', {
    role: props.message.role,
    contentLength: props.message.content?.length || 0,
    sourcesCount: parsedSources.value.length,
  });

  if (props.isStreaming) {
    console.debug('[MessageBubble] streaming started', {
      messageId: props.message.id,
    });
  }
});

watch(
  () => props.isStreaming,
  (streaming, wasStreaming) => {
    if (streaming && !wasStreaming) {
      console.debug('[MessageBubble] streaming started', {
        messageId: props.message.id,
      });
    } else if (!streaming && wasStreaming) {
      console.debug('[MessageBubble] streaming ended', {
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
  background: #2a2a4e;
  padding: 0.15rem 0.35rem;
  border-radius: 4px;
  font-size: 0.85em;
}

.markdown-body :deep(pre) {
  background: #12122a;
  border-radius: 8px;
  padding: 0.75rem;
  overflow-x: auto;
  margin: 0.5rem 0;
}

.markdown-body :deep(pre code) {
  background: none;
  padding: 0;
}

.markdown-body :deep(a) {
  color: #6b9fff;
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

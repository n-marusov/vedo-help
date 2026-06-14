<script setup lang="ts">
import type { Message, SourceRef } from '@/api/types';
import { marked } from 'marked';
import { computed, ref } from 'vue';

const props = defineProps<{
  message: Message;
  isStreaming?: boolean;
}>();

const sourcesExpanded = ref(false);

const renderedContent = computed(() => {
  if (!props.message.content) return '';
  return marked.parse(props.message.content, { async: false }) as string;
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
</script>

<template>
  <div
    class="message-bubble"
    :class="{
      'message-user': message.role === 'user',
      'message-assistant': message.role === 'assistant',
    }"
  >
    <div class="message-avatar">
      {{ message.role === 'user' ? '👤' : '🤖' }}
    </div>
    <div class="message-body">
      <div class="message-header">
        <span class="message-role">
          {{ message.role === 'user' ? 'You' : 'VEDO Assistant' }}
        </span>
        <span class="message-time">
          {{ new Date(message.created_at).toLocaleTimeString() }}
        </span>
      </div>

      <div class="message-content" v-if="message.content || isStreaming">
        <div v-if="message.content" class="markdown-body" v-html="renderedContent" />
        <div v-if="isStreaming && !message.content" class="typing-indicator">
          <span class="typing-dot" />
          <span class="typing-dot" />
          <span class="typing-dot" />
        </div>
      </div>

      <div
        v-if="parsedSources.length > 0 && message.role === 'assistant'"
        class="sources-section"
      >
        <button class="sources-toggle" @click="toggleSources">
          <span class="sources-icon">📚</span>
          {{ parsedSources.length }} source{{ parsedSources.length > 1 ? 's' : '' }}
          <span class="chevron" :class="{ expanded: sourcesExpanded }">▸</span>
        </button>
        <div v-if="sourcesExpanded" class="sources-list">
          <div
            v-for="(source, idx) in parsedSources"
            :key="idx"
            class="source-item"
          >
            <div class="source-header">
              <span class="source-doc">{{ source.document_name }}</span>
              <span class="source-relevance">
                {{ Math.round(source.relevance * 100) }}%
              </span>
            </div>
            <p class="source-text">{{ source.text }}</p>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.message-bubble {
  display: flex;
  gap: 0.75rem;
  padding: 1rem 1.5rem;
  animation: fadeIn 0.2s ease;
}

@keyframes fadeIn {
  from {
    opacity: 0;
    transform: translateY(4px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.message-user {
  flex-direction: row-reverse;
}

.message-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 1.1rem;
  background: #2a2a4e;
  flex-shrink: 0;
}

.message-body {
  max-width: 75%;
  min-width: 200px;
}

.message-user .message-body {
  background: #1a3a5c;
  border-radius: 12px 4px 12px 12px;
  padding: 0.75rem 1rem;
}

.message-assistant .message-body {
  background: #1e1e3a;
  border-radius: 4px 12px 12px 12px;
  padding: 0.75rem 1rem;
}

.message-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.35rem;
  font-size: 0.75rem;
}

.message-role {
  font-weight: 600;
  color: #8b8bbf;
}

.message-time {
  color: #5a5a7a;
  font-size: 0.7rem;
}

.message-content {
  font-size: 0.9rem;
  line-height: 1.5;
  color: #e0e0e0;
  word-break: break-word;
}

.markdown-body :deep(p) {
  margin: 0.35rem 0;
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

.typing-indicator {
  display: flex;
  gap: 4px;
  padding: 0.5rem 0;
}

.typing-dot {
  width: 8px;
  height: 8px;
  background: #6b9fff;
  border-radius: 50%;
  animation: typingBounce 1.4s infinite ease-in-out;
}

.typing-dot:nth-child(2) {
  animation-delay: 0.2s;
}

.typing-dot:nth-child(3) {
  animation-delay: 0.4s;
}

@keyframes typingBounce {
  0%,
  60%,
  100% {
    transform: translateY(0);
    opacity: 0.4;
  }
  30% {
    transform: translateY(-6px);
    opacity: 1;
  }
}

.sources-section {
  margin-top: 0.5rem;
  border-top: 1px solid #2a2a4e;
  padding-top: 0.5rem;
}

.sources-toggle {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  background: none;
  border: 1px solid #2a2a4e;
  border-radius: 6px;
  padding: 0.3rem 0.6rem;
  color: #8b8bbf;
  font-size: 0.8rem;
  cursor: pointer;
  transition: all 0.2s;
}

.sources-toggle:hover {
  background: #2a2a4e;
  color: #b0b0e0;
}

.sources-icon {
  font-size: 0.85rem;
}

.chevron {
  transition: transform 0.2s;
  font-size: 0.75rem;
}

.chevron.expanded {
  transform: rotate(90deg);
}

.sources-list {
  margin-top: 0.5rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.source-item {
  background: #12122a;
  border-radius: 6px;
  padding: 0.5rem 0.75rem;
  border-left: 3px solid #6b9fff;
}

.source-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.25rem;
}

.source-doc {
  font-size: 0.75rem;
  font-weight: 600;
  color: #8b8bbf;
}

.source-relevance {
  font-size: 0.7rem;
  color: #4caf50;
  font-weight: 600;
}

.source-text {
  font-size: 0.78rem;
  color: #a0a0c0;
  margin: 0;
  line-height: 1.4;
  display: -webkit-box;
  -webkit-line-clamp: 3;
  -webkit-box-orient: vertical;
  overflow: hidden;
}
</style>

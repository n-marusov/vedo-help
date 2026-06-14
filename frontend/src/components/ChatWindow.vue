<script setup lang="ts">
import { useChatStore } from '@/stores/chat';
import { useCollectionStore } from '@/stores/collections';
import { nextTick, onMounted, ref, watch } from 'vue';
import MessageBubble from './MessageBubble.vue';

const chatStore = useChatStore();
const collectionStore = useCollectionStore();

const inputText = ref('');
const messagesContainer = ref<HTMLElement | null>(null);

onMounted(() => {
  chatStore.fetchSessions();
  collectionStore.fetchCollections();
});

async function scrollToBottom() {
  await nextTick();
  if (messagesContainer.value) {
    messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight;
  }
}

watch(
  () => chatStore.messages.length,
  () => scrollToBottom(),
);

watch(
  () => chatStore.isLoading,
  (loading) => {
    if (loading) scrollToBottom();
  },
);

async function handleSend() {
  const text = inputText.value.trim();
  if (!text || chatStore.isLoading) return;

  const collectionId = collectionStore.activeCollectionId;
  if (!collectionId) {
    // Try to use first available collection
    if (collectionStore.collections.length > 0) {
      collectionStore.setActiveCollection(collectionStore.collections[0].id);
    } else {
      return;
    }
  }

  inputText.value = '';
  await chatStore.sendMessage(collectionStore.activeCollectionId as string, text);
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
}

async function handleNewChat() {
  chatStore.clearMessages();
  if (collectionStore.activeCollectionId) {
    await chatStore.createSession(collectionStore.activeCollectionId);
  }
}

function handleCancel() {
  chatStore.cancelStream();
}
</script>

<template>
  <div class="chat-window">
    <!-- Header -->
    <div class="chat-header">
      <div class="header-left">
        <div class="collection-selector">
          <select
            v-model="collectionStore.activeCollectionId"
            class="collection-select"
            @change="collectionStore.setActiveCollection(collectionStore.activeCollectionId)"
          >
            <option :value="null" disabled>Select a collection</option>
            <option v-for="col in collectionStore.collections" :key="col.id" :value="col.id">
              {{ col.name }}
            </option>
          </select>
        </div>
      </div>
      <div class="header-actions">
        <button class="btn-icon" title="New chat" @click="handleNewChat">
          <span class="icon">✏️</span>
        </button>
      </div>
    </div>

    <!-- Messages -->
    <div ref="messagesContainer" class="messages-area">
      <div v-if="chatStore.messages.length === 0" class="welcome-message">
        <div class="welcome-icon">💬</div>
        <h2>VEDO RAG Assistant</h2>
        <p>Select a collection and ask a question to get started.</p>
      </div>

      <MessageBubble
        v-for="(msg, idx) in chatStore.messages"
        :key="msg.id"
        :message="msg"
        :is-streaming="
          chatStore.isLoading && idx === chatStore.messages.length - 1 && msg.role === 'assistant'
        "
      />

      <div v-if="chatStore.error" class="error-banner">
        <span class="error-icon">⚠️</span>
        {{ chatStore.error }}
      </div>
    </div>

    <!-- Input area -->
    <div class="input-area">
      <button v-if="chatStore.isLoading" class="btn-cancel" @click="handleCancel">⏹ Cancel</button>
      <div class="input-row">
        <textarea
          v-model="inputText"
          class="chat-input"
          placeholder="Ask a question about your documents..."
          :disabled="chatStore.isLoading || !collectionStore.activeCollectionId"
          rows="1"
          @keydown="handleKeydown"
        />
        <button
          class="btn-send"
          :disabled="!inputText.trim() || chatStore.isLoading"
          @click="handleSend"
        >
          <span v-if="chatStore.isLoading" class="spinner" />
          <span v-else>➤</span>
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-window {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #0f0f23;
}

.chat-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1.5rem;
  border-bottom: 1px solid #2a2a4e;
  background: #1a1a2e;
  flex-shrink: 0;
}

.header-left {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.collection-select {
  background: #2a2a4e;
  color: #e0e0e0;
  border: 1px solid #3a3a5e;
  border-radius: 8px;
  padding: 0.4rem 0.75rem;
  font-size: 0.85rem;
  cursor: pointer;
  min-width: 200px;
}

.collection-select:focus {
  outline: none;
  border-color: #6b9fff;
}

.header-actions {
  display: flex;
  gap: 0.5rem;
}

.btn-icon {
  background: none;
  border: 1px solid #2a2a4e;
  border-radius: 8px;
  padding: 0.4rem 0.6rem;
  cursor: pointer;
  color: #8b8bbf;
  font-size: 0.9rem;
  transition: all 0.2s;
}

.btn-icon:hover {
  background: #2a2a4e;
  color: #e0e0e0;
}

.messages-area {
  flex: 1;
  overflow-y: auto;
  padding: 1rem 0;
}

.welcome-message {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: #5a5a7a;
  text-align: center;
  padding: 2rem;
}

.welcome-icon {
  font-size: 3rem;
  margin-bottom: 1rem;
}

.welcome-message h2 {
  margin: 0 0 0.5rem;
  color: #8b8bbf;
  font-size: 1.5rem;
}

.welcome-message p {
  margin: 0;
  font-size: 0.9rem;
}

.error-banner {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1.5rem;
  margin: 0.5rem 1.5rem;
  background: #3a1a1a;
  border: 1px solid #5a2a2a;
  border-radius: 8px;
  color: #ff6b6b;
  font-size: 0.85rem;
}

.input-area {
  flex-shrink: 0;
  border-top: 1px solid #2a2a4e;
  padding: 0.75rem 1.5rem;
  background: #1a1a2e;
}

.btn-cancel {
  display: block;
  margin-bottom: 0.5rem;
  background: #5a2a2a;
  border: 1px solid #7a3a3a;
  border-radius: 6px;
  padding: 0.3rem 0.75rem;
  color: #ff6b6b;
  font-size: 0.8rem;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-cancel:hover {
  background: #6a3a3a;
}

.input-row {
  display: flex;
  gap: 0.5rem;
  align-items: flex-end;
}

.chat-input {
  flex: 1;
  background: #2a2a4e;
  border: 1px solid #3a3a5e;
  border-radius: 10px;
  padding: 0.65rem 1rem;
  color: #e0e0e0;
  font-size: 0.9rem;
  font-family: inherit;
  resize: none;
  max-height: 120px;
  line-height: 1.4;
}

.chat-input:focus {
  outline: none;
  border-color: #6b9fff;
}

.chat-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-send {
  background: #4a6fff;
  border: none;
  border-radius: 10px;
  width: 40px;
  height: 40px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: white;
  font-size: 1.1rem;
  transition: background 0.2s;
  flex-shrink: 0;
}

.btn-send:hover:not(:disabled) {
  background: #5a7fff;
}

.btn-send:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.spinner {
  width: 16px;
  height: 16px;
  border: 2px solid rgba(255, 255, 255, 0.3);
  border-top-color: white;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>

<script setup>
import MessageBubble from '@/components/MessageBubble.vue';
import VButton from '@/components/ui/VButton.vue';
import VSelect from '@/components/ui/VSelect.vue';
import VThemeToggle from '@/components/ui/VThemeToggle.vue';
import { useChatStore } from '@/stores/chat';
import { useCollectionStore } from '@/stores/collections';
import { computed, nextTick, onMounted, ref, watch } from 'vue';

const chatStore = useChatStore();
const collectionStore = useCollectionStore();

const sidebarOpen = ref(false);
const inputText = ref('');
const messagesContainer = ref(null);
const textareaRef = ref(null);

onMounted(() => {
  chatStore.fetchSessions();
  collectionStore.fetchCollections();
});

// Close sidebar on session select for mobile
watch(
  () => chatStore.activeSessionId,
  () => {
    if (window.innerWidth < 768) {
      sidebarOpen.value = false;
    }
  },
);

function toggleSidebar() {
  sidebarOpen.value = !sidebarOpen.value;
}

function formatRelativeTime(dateStr) {
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const hours = Math.floor(diff / (1000 * 60 * 60));

  if (hours < 1) {
    const mins = Math.floor(diff / (1000 * 60));
    return `${mins}m ago`;
  }
  if (hours < 24) return `${hours}h ago`;
  if (hours < 168) return `${Math.floor(hours / 24)}d ago`;
  return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
}

function truncateTitle(title, maxLength = 35) {
  if (title.length <= maxLength) return title;
  return `${title.substring(0, maxLength)}...`;
}

async function handleSelectSession(sessionId) {
  await chatStore.loadSession(sessionId);
}

async function handleDeleteSession(sessionId, e) {
  e.stopPropagation();
  if (confirm('Delete this session?')) {
    await chatStore.deleteSession(sessionId);
  }
}

async function handleNewChat() {
  chatStore.clearMessages();
  if (collectionStore.activeCollectionId) {
    await chatStore.createSession(collectionStore.activeCollectionId);
  }
}

// Message sending logic
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

  let collectionId = collectionStore.activeCollectionId;
  if (!collectionId) {
    if (collectionStore.collections.length > 0) {
      collectionId = collectionStore.collections[0].id;
      collectionStore.setActiveCollection(collectionId);
    } else {
      return;
    }
  }

  inputText.value = '';
  resetTextareaHeight();
  await chatStore.sendMessage(collectionId, text);
}

function handleKeydown(e) {
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    handleSend();
  }
}

function autoResize() {
  const el = textareaRef.value;
  if (!el) return;
  el.style.height = 'auto';
  el.style.height = `${Math.min(el.scrollHeight, 200)}px`;
}

function resetTextareaHeight() {
  const el = textareaRef.value;
  if (el) el.style.height = 'auto';
}

function handleCancel() {
  chatStore.cancelStream();
}

const collectionOptions = computed(() =>
  collectionStore.collections.map((c) => ({
    label: c.name,
    value: c.id,
  })),
);

const hasInput = computed(() => inputText.value.trim().length > 0);
</script>

<template>
  <div class="chat-view" data-testid="chat-view">
    <!-- Mobile sidebar toggle -->
    <button
      class="sidebar-toggle"
      aria-label="Toggle sidebar"
      @click="toggleSidebar"
    >
      <svg
        aria-hidden="true"
        fill="none"
        height="20"
        viewBox="0 0 20 20"
        width="20"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M2.5 5H17.5"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-width="1.5"
        />
        <path
          d="M2.5 10H17.5"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-width="1.5"
        />
        <path
          d="M2.5 15H17.5"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-width="1.5"
        />
      </svg>
    </button>

    <!-- Session sidebar -->
    <aside
      class="session-sidebar"
      data-testid="session-sidebar"
      :class="{ 'session-sidebar--open': sidebarOpen }"
    >
      <div class="session-header">
        <span class="session-title">SESSIONS</span>
        <VButton variant="small" @click="handleNewChat">+ New</VButton>
      </div>

      <div v-if="chatStore.sessions.length === 0" class="session-empty">
        No sessions yet. Start a new chat!
      </div>

      <div v-else class="session-list">
        <button
          v-for="session in chatStore.sessions"
          :key="session.id"
          class="session-item"
          :class="{
            'session-item--active': session.id === chatStore.activeSessionId,
          }"
          @click="handleSelectSession(session.id)"
        >
          <div class="session-item-body">
            <span class="session-item-title">{{
              truncateTitle(session.title)
            }}</span>
            <span class="session-item-meta">
              {{ session.message_count }} msg ·
              {{ formatRelativeTime(session.updated_at) }}
            </span>
          </div>
          <button
            class="session-item-delete"
            @click="handleDeleteSession(session.id, $event)"
            title="Delete session"
          >
            🗑
          </button>
        </button>
      </div>
    </aside>

    <!-- Overlay for mobile sidebar -->
    <div
      v-if="sidebarOpen"
      class="sidebar-overlay"
      @click="sidebarOpen = false"
    />

    <!-- Main chat area -->
    <main class="chat-main">
      <!-- Toolbar -->
      <div class="toolbar" data-testid="chat-toolbar">
        <div class="toolbar-left">
          <VSelect
            v-model="collectionStore.activeCollectionId"
            :options="collectionOptions"
            placeholder="Select a collection..."
            class="toolbar-select"
            @update:model-value="collectionStore.setActiveCollection"
          />
        </div>
        <div class="toolbar-right">
          <VThemeToggle />
          <VButton variant="ghost" @click="handleNewChat">✎ New chat</VButton>
        </div>
      </div>

      <!-- Messages area -->
      <div
        ref="messagesContainer"
        class="messages-area"
        data-testid="messages-area"
      >
        <!-- Welcome block -->
        <div v-if="chatStore.messages.length === 0" class="welcome-screen">
          <div class="welcome-content" data-testid="welcome-message">
            <span class="welcome-icon">💬</span>
            <h2 class="welcome-title">VEDO RAG Assistant</h2>
            <p class="welcome-subtitle">
              Select a collection and ask a question.
            </p>
          </div>
        </div>

        <!-- Messages -->
        <div v-else class="messages-list">
          <MessageBubble
            v-for="(msg, idx) in chatStore.messages"
            :key="msg.id"
            :message="msg"
            :index="idx"
            :is-streaming="
              chatStore.isLoading &&
              idx === chatStore.messages.length - 1 &&
              msg.role === 'assistant'
            "
          />
        </div>

        <!-- Error banner -->
        <div
          v-if="chatStore.error"
          class="error-banner"
          data-testid="error-banner"
        >
          <svg
            aria-hidden="true"
            class="error-icon"
            fill="none"
            viewBox="0 0 16 16"
            xmlns="http://www.w3.org/2000/svg"
          >
            <circle cx="8" cy="8" fill="currentColor" opacity="0.2" r="7" />
            <text
              dominant-baseline="central"
              fill="currentColor"
              font-size="10"
              font-weight="700"
              text-anchor="middle"
              x="8"
              y="8.5"
            >
              !
            </text>
          </svg>
          {{ chatStore.error }}
        </div>
      </div>

      <!-- Composer -->
      <div class="composer" data-testid="composer">
        <div class="composer-input-wrap">
          <textarea
            ref="textareaRef"
            v-model="inputText"
            class="composer-textarea"
            data-testid="chat-input"
            :placeholder="
              collectionStore.activeCollectionId
                ? 'Ask a question about your documents...'
                : 'Select a collection to get started'
            "
            :disabled="
              chatStore.isLoading || !collectionStore.activeCollectionId
            "
            rows="1"
            @input="autoResize"
            @keydown="handleKeydown"
          />
          <button
            v-if="chatStore.isLoading"
            class="composer-btn composer-btn--cancel"
            data-testid="btn-cancel"
            title="Cancel"
            @click="handleCancel"
          >
            <svg
              aria-hidden="true"
              fill="none"
              height="18"
              viewBox="0 0 18 18"
              width="18"
              xmlns="http://www.w3.org/2000/svg"
            >
              <rect
                fill="currentColor"
                height="12"
                rx="2"
                width="12"
                x="3"
                y="3"
              />
            </svg>
          </button>
          <button
            v-else
            class="composer-btn composer-btn--send"
            data-testid="btn-send"
            :class="{ 'composer-btn--active': hasInput }"
            :disabled="!hasInput || chatStore.isLoading"
            title="Send message"
            @click="handleSend"
          >
            <svg
              aria-hidden="true"
              fill="none"
              height="18"
              viewBox="0 0 18 18"
              width="18"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M2 16L16 9L2 2L2 7.33333L12 9L2 10.6667L2 16Z"
                fill="currentColor"
              />
            </svg>
          </button>
        </div>
        <p
          v-if="
            !collectionStore.activeCollectionId &&
            collectionStore.collections.length === 0
          "
          class="composer-hint"
        >
          Upload documents in the Admin panel to create a collection.
        </p>
      </div>
    </main>
  </div>
</template>

<style scoped>
/* ===================================================================
   Chat View — Pencil Design Implementation
   =================================================================== */

.chat-view {
  display: flex;
  height: 100%;
  overflow: hidden;
  position: relative;
  padding: var(--space-6);
  gap: var(--space-6);
}

/* ===== Session Sidebar ===== */

.session-sidebar {
  width: 312px;
  min-width: 312px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  padding: var(--space-5);
  overflow: hidden;
}

.session-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.session-title {
  font-size: var(--font-size-xs);
  font-weight: 600;
  color: var(--color-muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.session-empty {
  padding: var(--space-6) 0;
  text-align: center;
  color: var(--color-muted-foreground);
  font-size: var(--font-size-sm);
}

.session-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.session-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  width: 100%;
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  padding: 14px;
  cursor: pointer;
  text-align: left;
  transition: all var(--transition-fast);
}

.session-item:hover {
  border-color: var(--color-primary);
  opacity: 0.9;
}

.session-item--active {
  background: var(--color-accent);
  border-color: var(--color-primary);
}

.session-item-body {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.session-item-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-item-meta {
  font-size: 10px;
  color: var(--color-muted-foreground);
}

.session-item-delete {
  background: none;
  border: none;
  padding: 0;
  font-size: 10px;
  cursor: pointer;
  color: var(--color-muted-foreground);
  flex-shrink: 0;
  line-height: 1;
  opacity: 0;
  transition: opacity var(--transition-fast);
}

.session-item:hover .session-item-delete {
  opacity: 1;
}

.session-item-delete:hover {
  color: var(--color-destructive);
}

/* ===== Main Chat Area ===== */

.chat-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-2xl);
  overflow: hidden;
}

/* ===== Toolbar ===== */

.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 72px;
  padding: var(--space-4) var(--space-5);
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.toolbar-left {
  display: flex;
  align-items: center;
}

.toolbar-select {
  width: 360px;
}

.toolbar-right {
  display: flex;
  align-items: center;
}

/* ===== Messages Area ===== */

.messages-area {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  padding: 28px;
  gap: 18px;
  scroll-behavior: smooth;
}

.messages-list {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  width: 100%;
  max-width: 820px;
  margin: 0 auto;
}

/* ===== Welcome Screen ===== */

.welcome-screen {
  display: flex;
  align-items: center;
  justify-content: center;
  flex: 1;
}

.welcome-content {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
  max-width: 420px;
}

.welcome-icon {
  font-size: 26px;
  line-height: 1;
  margin-bottom: var(--space-3);
  color: var(--color-primary);
}

.welcome-title {
  margin: 0 0 var(--space-2);
  font-size: var(--font-size-xl);
  font-weight: 700;
  color: var(--color-foreground);
}

.welcome-subtitle {
  margin: 0;
  font-size: var(--font-size-xs);
  color: var(--color-muted-foreground);
  line-height: 1.5;
}

/* ===== Error Banner ===== */

.error-banner {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  margin: 0 auto;
  width: 100%;
  max-width: 820px;
  background: color-mix(in srgb, var(--color-destructive) 15%, transparent);
  border: 1px solid
    color-mix(in srgb, var(--color-destructive) 40%, transparent);
  border-radius: var(--radius-md);
  color: var(--color-destructive);
  font-size: var(--font-size-sm);
}

.error-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

/* ===== Composer ===== */

.composer {
  flex-shrink: 0;
  padding: var(--space-5);
  border-top: 1px solid var(--color-border);
}

.composer-input-wrap {
  display: flex;
  gap: var(--space-2);
  align-items: flex-end;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  background: var(--color-secondary);
  padding: var(--space-2);
  transition: border-color var(--transition-fast);
}

.composer-input-wrap:focus-within {
  border-color: var(--color-primary);
}

.composer-textarea {
  flex: 1;
  background: transparent;
  border: none;
  padding: var(--space-2);
  color: var(--color-foreground);
  font-size: 0.9rem;
  font-family: var(--font-family);
  resize: none;
  min-height: var(--input-min-height);
  max-height: 200px;
  line-height: 1.5;
  outline: none;
}

.composer-textarea::placeholder {
  color: var(--color-muted-foreground);
}

.composer-textarea:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.composer-btn {
  border: none;
  border-radius: var(--radius-md);
  width: 38px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}

.composer-btn--send {
  background: transparent;
  color: var(--color-muted-foreground);
}

.composer-btn--send:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.composer-btn--active {
  color: var(--color-primary-foreground);
  background: var(--color-primary);
}

.composer-btn--active:hover {
  opacity: 0.9;
}

.composer-btn--cancel {
  background: color-mix(in srgb, var(--color-destructive) 20%, transparent);
  color: var(--color-destructive);
}

.composer-btn--cancel:hover {
  background: color-mix(in srgb, var(--color-destructive) 35%, transparent);
}

.composer-hint {
  margin: var(--space-2) 0 0;
  text-align: center;
  font-size: var(--font-size-2xs);
  color: var(--color-muted-foreground);
}

/* ===== Sidebar Toggle (Mobile) ===== */

.sidebar-toggle {
  display: none;
  position: fixed;
  top: 12px;
  left: 12px;
  z-index: 90;
  background: var(--color-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  padding: 0.35rem;
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.sidebar-toggle:hover {
  background: var(--color-border);
  color: var(--color-foreground);
}

/* ===== Sidebar Overlay (Mobile) ===== */

.sidebar-overlay {
  display: none;
  position: fixed;
  inset: 0;
  z-index: 80;
  background: rgba(0, 0, 0, 0.5);
}

/* ===== Scrollbar Styling ===== */

.messages-area::-webkit-scrollbar,
.session-list::-webkit-scrollbar {
  width: 6px;
}

.messages-area::-webkit-scrollbar-track,
.session-list::-webkit-scrollbar-track {
  background: transparent;
}

.messages-area::-webkit-scrollbar-thumb,
.session-list::-webkit-scrollbar-thumb {
  background: var(--color-border);
  border-radius: 3px;
}

.messages-area::-webkit-scrollbar-thumb:hover,
.session-list::-webkit-scrollbar-thumb:hover {
  background: var(--color-input);
}

/* ===== Mobile Responsive ===== */

@media (max-width: 768px) {
  .chat-view {
    padding: 0;
    gap: 0;
  }

  .sidebar-toggle {
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .session-sidebar {
    position: fixed;
    top: 0;
    left: 0;
    bottom: 0;
    z-index: 85;
    width: 312px;
    min-width: 312px;
    transform: translateX(-100%);
    transition: transform 0.25s ease;
    border-radius: 0;
    border-right: 1px solid var(--color-border);
    padding: var(--space-4);
  }

  .session-sidebar--open {
    transform: translateX(0);
  }

  .sidebar-overlay {
    display: block;
  }

  .chat-main {
    border-radius: 0;
    border: none;
  }

  .toolbar-select {
    width: auto;
  }

  .composer-textarea {
    font-size: 1rem;
    min-height: 44px;
  }

  .composer {
    padding: var(--space-3);
  }

  .composer-input-wrap {
    padding: var(--space-2);
  }

  .composer-btn {
    width: 44px;
    height: 44px;
  }

  .messages-area {
    padding: var(--space-4);
  }

  .messages-list {
    max-width: 100%;
  }

  .toolbar {
    height: auto;
    padding: var(--space-3);
    flex-wrap: wrap;
    gap: var(--space-2);
  }

  .welcome-title {
    font-size: var(--font-size-lg);
  }

  .welcome-subtitle {
    font-size: var(--font-size-xs);
  }

  .welcome-icon {
    font-size: 22px;
  }
}

@media (max-width: 480px) {
  .session-sidebar {
    width: 100%;
    min-width: 100%;
  }

  .messages-area {
    padding: var(--space-3);
    gap: var(--space-3);
  }

  .composer {
    padding: var(--space-2);
    padding-bottom: max(
      var(--space-2),
      env(safe-area-inset-bottom, var(--space-2))
    );
  }

  .toolbar {
    padding: var(--space-2);
  }

  .welcome-screen {
    padding: var(--space-4);
  }

  .error-banner {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    font-size: var(--font-size-xs);
    max-width: 100%;
  }
}
</style>

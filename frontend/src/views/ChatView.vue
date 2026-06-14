<script setup lang="ts">
import ChatWindow from '@/components/ChatWindow.vue';
import { useChatStore } from '@/stores/chat';
import { useCollectionStore } from '@/stores/collections';
import { onMounted } from 'vue';

const chatStore = useChatStore();
const collectionStore = useCollectionStore();

onMounted(() => {
  chatStore.fetchSessions();
  collectionStore.fetchCollections();
});

function formatDate(dateStr: string): string {
  const date = new Date(dateStr);
  const now = new Date();
  const diff = now.getTime() - date.getTime();
  const hours = Math.floor(diff / (1000 * 60 * 60));

  if (hours < 24) {
    return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  }
  if (hours < 168) {
    return `${Math.floor(hours / 24)}d ago`;
  }
  return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
}

function truncateTitle(title: string, maxLength = 35): string {
  if (title.length <= maxLength) return title;
  return `${title.substring(0, maxLength)}...`;
}

async function handleSelectSession(sessionId: string) {
  await chatStore.loadSession(sessionId);
}

async function handleDeleteSession(sessionId: string, e: Event) {
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
</script>

<template>
  <div class="chat-view">
    <!-- Session sidebar -->
    <aside class="session-sidebar">
      <div class="session-header">
        <h3 class="session-title">Sessions</h3>
        <button class="btn-new-session" @click="handleNewChat" title="New session">
          + New
        </button>
      </div>

      <div v-if="chatStore.sessions.length === 0" class="session-empty">
        <p>No sessions yet. Start a new chat!</p>
      </div>

      <div v-else class="session-list">
        <button
          v-for="session in chatStore.sessions"
          :key="session.id"
          class="session-item"
          :class="{ active: session.id === chatStore.activeSessionId }"
          @click="handleSelectSession(session.id)"
        >
          <div class="session-item-content">
            <span class="session-item-title">
              {{ truncateTitle(session.title) }}
            </span>
            <span class="session-item-meta">
              {{ session.message_count }} msg · {{ formatDate(session.updated_at) }}
            </span>
          </div>
          <button
            class="btn-delete-session"
            @click="handleDeleteSession(session.id, $event)"
            title="Delete session"
          >
            ×
          </button>
        </button>
      </div>
    </aside>

    <!-- Main chat area -->
    <main class="chat-main">
      <ChatWindow />
    </main>
  </div>
</template>

<style scoped>
.chat-view {
  display: flex;
  height: 100%;
  overflow: hidden;
}

.session-sidebar {
  width: 260px;
  min-width: 260px;
  background: #16162e;
  border-right: 1px solid #2a2a4e;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.session-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #2a2a4e;
  flex-shrink: 0;
}

.session-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  color: #8b8bbf;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.btn-new-session {
  background: #4a6fff;
  border: none;
  border-radius: 6px;
  padding: 0.3rem 0.6rem;
  color: white;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-new-session:hover {
  background: #5a7fff;
}

.session-empty {
  padding: 2rem 1rem;
  text-align: center;
  color: #5a5a7a;
  font-size: 0.85rem;
}

.session-list {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
}

.session-item {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  width: 100%;
  background: none;
  border: 1px solid transparent;
  border-radius: 8px;
  padding: 0.6rem 0.75rem;
  margin-bottom: 0.25rem;
  cursor: pointer;
  text-align: left;
  transition: all 0.2s;
}

.session-item:hover {
  background: #1e1e3e;
  border-color: #2a2a4e;
}

.session-item.active {
  background: #1a2a4e;
  border-color: #4a6fff;
}

.session-item-content {
  flex: 1;
  min-width: 0;
}

.session-item-title {
  display: block;
  font-size: 0.82rem;
  color: #c0c0e0;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-item-meta {
  display: block;
  font-size: 0.7rem;
  color: #5a5a7a;
  margin-top: 0.15rem;
}

.btn-delete-session {
  background: none;
  border: none;
  color: #5a5a7a;
  font-size: 1.1rem;
  cursor: pointer;
  padding: 0.15rem 0.25rem;
  border-radius: 4px;
  opacity: 0;
  transition: all 0.2s;
}

.session-item:hover .btn-delete-session {
  opacity: 1;
}

.btn-delete-session:hover {
  color: #ff6b6b;
  background: #3a1a1a;
}

.chat-main {
  flex: 1;
  overflow: hidden;
}

/* Mobile responsive */
@media (max-width: 768px) {
  .session-sidebar {
    width: 100%;
    min-width: 100%;
    max-height: 200px;
    border-right: none;
    border-bottom: 1px solid #2a2a4e;
  }

  .chat-view {
    flex-direction: column;
  }
}
</style>

<script setup>
import CollectionSelector from '@/components/CollectionSelector.vue';
import MessageBubble from '@/components/MessageBubble.vue';
import VBadge from '@/components/ui/VBadge.vue';
import VButton from '@/components/ui/VButton.vue';
import VDialog from '@/components/ui/VDialog.vue';
import { useChatStore } from '@/stores/chat';
import { useCollectionStore } from '@/stores/collections';
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';

const chatStore = useChatStore();
const collectionStore = useCollectionStore();

const sidebarOpen = ref(false);
const searchDialogOpen = ref(false);
const renameDialogOpen = ref(false);
const renameSessionTarget = ref(null);
const renameInput = ref('');
const inputText = ref('');
const messagesContainer = ref(null);
const textareaRef = ref(null);
const exportFormat = ref('md');
const newSessionDropdownOpen = ref(false);
const newSessionTriggerRef = ref(null);
const newSessionDropdownRef = ref(null);
const newSessionDropdownStyle = ref({});

const exportFormatOptions = [
  { label: 'Markdown', value: 'md' },
  { label: 'JSON', value: 'json' },
];

onMounted(() => {
  document.addEventListener('click', handleNewSessionClickOutside);
  document.addEventListener('keydown', handleNewSessionKeydown);
  window.addEventListener('resize', updateNewSessionDropdownPosition);
  window.addEventListener('scroll', updateNewSessionDropdownPosition, true);

  // Fetch sessions and collections, then check for pending pipeline
  console.warn(
    '[FIX] ChatView.onMounted: starting fetchSessions -> fetchCollections -> checkPendingPipeline chain',
  );
  chatStore.fetchSessions().finally(() => {
    console.warn('[FIX] ChatView.onMounted: fetchSessions completed, starting fetchCollections');
    collectionStore.fetchCollections().finally(() => {
      console.warn(
        '[FIX] ChatView.onMounted: fetchCollections completed, calling checkPendingPipeline',
      );
      chatStore.checkPendingPipeline();
    });
  });
});

onUnmounted(() => {
  document.removeEventListener('click', handleNewSessionClickOutside);
  document.removeEventListener('keydown', handleNewSessionKeydown);
  window.removeEventListener('resize', updateNewSessionDropdownPosition);
  window.removeEventListener('scroll', updateNewSessionDropdownPosition, true);
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

function openSearchDialog() {
  searchDialogOpen.value = true;
  chatStore.setSearchQuery('');
}

function closeSearchDialog() {
  searchDialogOpen.value = false;
  chatStore.setSearchQuery('');
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

function handleRenameSession(session) {
  renameSessionTarget.value = session;
  renameInput.value = session.title;
  renameDialogOpen.value = true;
}

async function confirmRename() {
  if (!renameSessionTarget.value) return;
  const title = renameInput.value.trim();
  if (!title) {
    renameDialogOpen.value = false;
    renameSessionTarget.value = null;
    return;
  }
  await chatStore.renameSession(renameSessionTarget.value.id, title);
  renameDialogOpen.value = false;
  renameSessionTarget.value = null;
}

function cancelRename() {
  renameDialogOpen.value = false;
  renameSessionTarget.value = null;
}

async function togglePin(sessionId) {
  await chatStore.togglePinSession(sessionId);
}

function updateNewSessionDropdownPosition() {
  if (!newSessionDropdownOpen.value || !newSessionTriggerRef.value) return;
  const rect = newSessionTriggerRef.value.getBoundingClientRect();
  newSessionDropdownStyle.value = {
    left: `${rect.left}px`,
    minWidth: `${rect.width}px`,
    top: `${rect.bottom + 4}px`,
  };
}

async function openNewSessionDropdown() {
  newSessionDropdownOpen.value = true;
  await nextTick();
  updateNewSessionDropdownPosition();
  console.debug(
    '[ChatView.newSession] opened collection dropdown count=%d',
    collectionStore.collections.length,
  );
}

function closeNewSessionDropdown() {
  newSessionDropdownOpen.value = false;
}

async function toggleNewSessionDropdown() {
  if (chatStore.isLoading || chatStore.isSessionLoading) return;
  if (collectionStore.collections.length === 0) {
    console.debug('[ChatView.newSession] no collections available, clearing active session');
    chatStore.clearMessages();
    return;
  }
  if (newSessionDropdownOpen.value) {
    closeNewSessionDropdown();
  } else {
    await openNewSessionDropdown();
  }
}

async function handleNewSessionForCollection(collectionId) {
  closeNewSessionDropdown();
  collectionStore.setActiveCollection(collectionId);
  chatStore.clearMessages();
  console.debug('[ChatView.newSession] preparing session with collection=%s', collectionId);
  // Session will be created on first message send
}

function handleNewSessionClickOutside(e) {
  if (!newSessionDropdownOpen.value) return;
  const target = e.target;
  if (
    newSessionTriggerRef.value?.contains(target) ||
    newSessionDropdownRef.value?.contains(target)
  ) {
    return;
  }
  closeNewSessionDropdown();
}

function handleNewSessionKeydown(e) {
  if (e.key === 'Escape') {
    closeNewSessionDropdown();
  }
}

function isNewSessionCollectionActive(collectionId) {
  return collectionId === collectionStore.activeCollectionId;
}

async function handleNewChat() {
  await toggleNewSessionDropdown();
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

  // Ensure a session exists before sending
  if (!chatStore.activeSessionId) {
    const session = await chatStore.createSession(collectionId);
    if (!session) {
      console.warn('[ChatView] failed to create session, sending without one');
      // Continue anyway — backend will handle session-less queries
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

function handleEditMessage({ id }) {
  // Edit is handled inline in MessageBubble (startEdit/saveEdit)
}

function handleCopyMessage({ id }) {
  chatStore.copyMessage(id);
}

async function handleRegenerateMessage({ id }) {
  await chatStore.regenerateMessage(id);
}

function handleSaveEdit({ id, content }) {
  if (chatStore.activeSessionId) {
    chatStore.editMessage(chatStore.activeSessionId, id, content);
  }
}

async function handleExport() {
  if (chatStore.activeSessionId) {
    console.debug('[ChatView] export format=%s', exportFormat.value);
    await chatStore.exportSession(chatStore.activeSessionId, exportFormat.value);
  }
}

function handleCancel() {
  chatStore.cancelStream();
}

// removed: collectionOptions — replaced by CollectionSelector component

watch(
  () => collectionStore.collections.length,
  () => {
    if (newSessionDropdownOpen.value) {
      nextTick(() => updateNewSessionDropdownPosition());
    }
  },
);

const activeSession = computed(
  () => chatStore.sessions.find((s) => s.id === chatStore.activeSessionId) || null,
);

const activeCollectionName = computed(() => {
  if (!collectionStore.activeCollectionId) return '';
  const col = collectionStore.collections.find((c) => c.id === collectionStore.activeCollectionId);
  return col?.name || '';
});

const hasVisibleStreamingPlaceholder = computed(() => {
  if (!chatStore.isLoading || chatStore.messages.length === 0) return false;
  const last = chatStore.messages[chatStore.messages.length - 1];
  return last?.role === 'assistant' && !last.content;
});

const isActiveSessionPipeline = computed(
  () =>
    chatStore.isLoading &&
    !!chatStore.activeSessionId &&
    chatStore.pipelineSessionId === chatStore.activeSessionId,
);

const shouldShowPipelineStatusBar = computed(
  () =>
    isActiveSessionPipeline.value &&
    !!chatStore.pipelineStageLabel &&
    !hasVisibleStreamingPlaceholder.value,
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
      :class="[
        { 'session-sidebar--open': sidebarOpen },
        { 'session-sidebar--collapsed': chatStore.sidebarCollapsed },
      ]"
    >
      <!-- Expand button when sidebar is collapsed -->
      <button
        v-if="chatStore.sidebarCollapsed"
        class="sidebar-expand-btn"
        data-testid="sidebar-expand-btn"
        title="Expand sidebar"
        @click="chatStore.toggleSidebarCollapsed"
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
            d="M7 4L12 9L7 14"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.5"
          />
        </svg>
      </button>

      <!-- New session button on collapsed sidebar -->
      <template v-if="chatStore.sidebarCollapsed">
        <button
          class="sidebar-search-btn"
          data-testid="sidebar-search-btn"
          title="Search sessions"
          @click="openSearchDialog"
        >
          <svg
            aria-hidden="true"
            fill="none"
            height="16"
            viewBox="0 0 16 16"
            width="16"
            xmlns="http://www.w3.org/2000/svg"
          >
            <circle
              cx="7"
              cy="7"
              r="5.5"
              stroke="currentColor"
              stroke-width="1.5"
            />
            <path
              d="M11 11L14.5 14.5"
              stroke="currentColor"
              stroke-linecap="round"
              stroke-width="1.5"
            />
          </svg>
        </button>
        <button
          class="sidebar-new-session-btn"
          data-testid="sidebar-new-session-btn"
          title="New session"
          @click="handleNewChat"
        >
          <span class="sidebar-new-session-icon">+</span>
        </button>
      </template>

      <div class="session-header">
        <span class="session-title">HISTORY</span>
        <div class="session-header-actions">
          <button
            class="session-header-btn"
            data-testid="session-search-toggle"
            title="Search sessions"
            @click="openSearchDialog"
          >
            <svg
              aria-hidden="true"
              fill="none"
              height="16"
              viewBox="0 0 16 16"
              width="16"
              xmlns="http://www.w3.org/2000/svg"
            >
              <circle
                cx="7"
                cy="7"
                r="5.5"
                stroke="currentColor"
                stroke-width="1.5"
              />
              <path
                d="M11 11L14.5 14.5"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-width="1.5"
              />
            </svg>
          </button>
          <button
            class="session-header-btn"
            data-testid="sidebar-collapse-btn"
            :title="
              chatStore.sidebarCollapsed ? 'Expand sidebar' : 'Collapse sidebar'
            "
            @click="chatStore.toggleSidebarCollapsed"
          >
            <svg
              aria-hidden="true"
              fill="none"
              height="16"
              viewBox="0 0 16 16"
              width="16"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                v-if="!chatStore.sidebarCollapsed"
                d="M10 3L6 8L10 13"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1.5"
              />
              <path
                v-else
                d="M6 3L10 8L6 13"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1.5"
              />
            </svg>
          </button>
        </div>
      </div>

      <!-- New session button with collection dropdown -->
      <div class="session-new-wrapper">
        <button
          ref="newSessionTriggerRef"
          class="session-new-trigger"
          data-testid="btn-new-chat"
          type="button"
          :aria-expanded="newSessionDropdownOpen"
          aria-haspopup="listbox"
          :disabled="chatStore.isLoading || chatStore.isSessionLoading"
          @click="handleNewChat"
        >
          <span class="session-new-label-group">
            <span class="session-new-plus">+</span>
            <span class="session-new-label">New session</span>
          </span>
          <span
            class="session-new-chevron"
            :class="{ 'session-new-chevron--open': newSessionDropdownOpen }"
            >▾</span
          >
        </button>
      </div>

      <Teleport to="body">
        <div
          v-if="newSessionDropdownOpen"
          ref="newSessionDropdownRef"
          class="session-new-dropdown"
          data-testid="new-session-collection-dropdown"
          :style="newSessionDropdownStyle"
        >
          <div class="session-new-caption">Choose collection</div>
          <button
            v-for="collection in collectionStore.collections"
            :key="collection.id"
            class="session-new-option"
            :class="{
              'session-new-option--selected': isNewSessionCollectionActive(
                collection.id,
              ),
            }"
            data-testid="new-session-collection-option"
            type="button"
            @click="handleNewSessionForCollection(collection.id)"
          >
            <span class="session-new-check">{{
              isNewSessionCollectionActive(collection.id) ? "✓" : ""
            }}</span>
            <span class="session-new-option-label">{{ collection.name }}</span>
            <span
              v-if="collection.document_count !== undefined"
              class="session-new-option-count"
              >{{ collection.document_count }}</span
            >
          </button>
        </div>
      </Teleport>

      <div v-if="chatStore.filteredSessions.length === 0" class="session-empty">
        No sessions yet. Start a new chat!
      </div>

      <div v-else class="session-list">
        <template
          v-for="group in chatStore.filteredSessionsByPeriod"
          :key="group.label ?? 'pinned'"
        >
          <span
            v-if="group.label"
            class="session-section-header"
            data-testid="session-section-header"
            >{{ group.label }}</span
          >
          <div
            v-for="session in group.sessions"
            :key="session.id"
            class="session-item"
            :class="[
              {
                'session-item--active':
                  session.id === chatStore.activeSessionId,
              },
              { 'session-item--pinned': session.pinned },
            ]"
            :data-pinned="session.pinned ? 'true' : 'false'"
            data-testid="session-item"
            @click="handleSelectSession(session.id)"
            role="button"
            tabindex="0"
            @keydown.enter="handleSelectSession(session.id)"
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
            <div class="session-item-actions">
              <button
                class="session-action-btn"
                data-testid="session-pin-btn"
                title="Pin session"
                @click.stop="togglePin(session.id)"
              >
                <svg
                  aria-hidden="true"
                  fill="none"
                  height="12"
                  viewBox="0 0 12 12"
                  width="12"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    d="M7.5 1L9.5 3L8 4.5L10 7L9 8L6 5L4 10L2 10L3 8L1 7L4 4.5L3 3L5 1L6.5 2.5L7.5 1Z"
                    fill="currentColor"
                  />
                </svg>
              </button>
              <button
                class="session-action-btn"
                data-testid="session-rename-btn"
                title="Rename session"
                @click.stop="handleRenameSession(session)"
              >
                <svg
                  aria-hidden="true"
                  fill="none"
                  height="12"
                  viewBox="0 0 12 12"
                  width="12"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    d="M8.5 1L11 3.5L4 10.5L1 11L1.5 8L8.5 1Z"
                    stroke="currentColor"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    stroke-width="1.2"
                  />
                </svg>
              </button>
              <button
                class="session-action-btn session-action-btn--delete"
                data-testid="session-delete-btn"
                title="Delete session"
                @click.stop="handleDeleteSession(session.id, $event)"
              >
                <svg
                  aria-hidden="true"
                  fill="none"
                  height="12"
                  viewBox="0 0 12 12"
                  width="12"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    d="M2 3H10"
                    stroke="currentColor"
                    stroke-linecap="round"
                    stroke-width="1.2"
                  />
                  <path
                    d="M4 2H8"
                    stroke="currentColor"
                    stroke-linecap="round"
                    stroke-width="1.2"
                  />
                  <path
                    d="M3 4L3.5 9.5C3.5 10.3 4.2 11 5 11H7C7.8 11 8.5 10.3 8.5 9.5L9 4"
                    stroke="currentColor"
                    stroke-linecap="round"
                    stroke-width="1.2"
                  />
                </svg>
              </button>
            </div>
          </div>
        </template>
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
          <div
            v-if="activeSession"
            class="toolbar-badges"
            data-testid="toolbar-badges"
          >
            <h1
              class="toolbar-session-title"
              data-testid="toolbar-session-title"
              >{{ activeSession.title }}</h1>
            <VBadge
              v-if="activeCollectionName"
              size="sm"
              variant="info"
              data-testid="toolbar-collection-badge"
              class="toolbar-collection-tag"
              >{{ activeCollectionName }}</VBadge
            >
          </div>
          <div
            v-else-if="activeCollectionName"
            class="toolbar-badges"
            data-testid="toolbar-badges"
          >
            <VBadge
              size="sm"
              variant="info"
              data-testid="toolbar-collection-badge"
              >{{ activeCollectionName }}</VBadge
            >
          </div>
          <CollectionSelector
            v-else
            v-model="collectionStore.activeCollectionId"
            :collections="collectionStore.collections"
            :active-collection-id="collectionStore.activeCollectionId"
            placeholder="Select a collection..."
          />
        </div>
        <div class="toolbar-right">
          <button
            v-if="chatStore.activeSessionId"
            class="toolbar-icon-btn"
            data-testid="export-btn"
            :disabled="chatStore.isExporting"
            title="Export session"
            @click="handleExport"
          >
            <svg
              fill="none"
              height="18"
              viewBox="0 0 18 18"
              width="18"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M9 3V12M9 12L6 9M9 12L12 9"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <path
                d="M3 12V14C3 15.1 3.9 16 5 16H13C14.1 16 15 15.1 15 14V12"
                stroke="currentColor"
                stroke-width="1.5"
                stroke-linecap="round"
              />
            </svg>
          </button>
        </div>
      </div>

      <!-- Messages area -->
      <div
        ref="messagesContainer"
        class="messages-area"
        data-testid="messages-area"
      >
        <!-- Welcome block (no session selected) -->
        <div
          v-if="!chatStore.activeSessionId"
          class="welcome-screen"
          data-testid="welcome-message"
        >
          <div class="welcome-content">
            <span class="welcome-icon">💬</span>
            <h2 class="welcome-title">VEDO RAG Assistant</h2>
            <p class="welcome-subtitle">
              Select a collection and ask a question.
            </p>
          </div>
        </div>

        <!-- Empty active session -->
        <div
          v-else-if="chatStore.messages.length === 0"
          data-testid="session-empty-messages"
          class="welcome-screen"
        >
          <div class="welcome-content">
            <span class="welcome-icon">💬</span>
            <h2 class="welcome-title">No messages yet</h2>
            <p class="welcome-subtitle">
              Ask a question to start the conversation.
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
              isActiveSessionPipeline &&
              idx === chatStore.messages.length - 1 &&
              msg.role === 'assistant'
            "
            @edit="handleEditMessage"
            @save-edit="handleSaveEdit"
            @cancel-edit="() => {}"
            @copy="handleCopyMessage"
            @regenerate="handleRegenerateMessage"
            :pipeline-stage-label="chatStore.pipelineStageLabel"
          />
        </div>

        <div
          v-if="shouldShowPipelineStatusBar"
          class="pipeline-status-bar"
          data-testid="pipeline-status-bar"
          aria-live="polite"
        >
          <span class="pipeline-status-dot" aria-hidden="true" />
          <span class="pipeline-status-label">{{ chatStore.pipelineStageLabel }}</span>
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

    <!-- Rename session dialog -->
    <VDialog
      :open="renameDialogOpen"
      title="Rename session"
      description="Enter a new name for this session."
      confirmText="Save"
      cancelText="Cancel"
      @close="cancelRename"
      @confirm="confirmRename"
    >
      <div class="rename-dialog-body">
        <input
          v-model="renameInput"
          class="rename-dialog-input"
          data-testid="session-rename-input"
          type="text"
          placeholder="Session name"
          @keydown.enter="confirmRename"
        />
      </div>
    </VDialog>

    <!-- Search sessions dialog -->
    <VDialog
      :open="searchDialogOpen"
      title="Search Sessions"
      cancelText="Close"
      @close="closeSearchDialog"
      @confirm="closeSearchDialog"
    >
      <template #actions>
        <VButton
          variant="outline"
          data-testid="btn-dialog-close"
          @click="closeSearchDialog"
        >
          Close
        </VButton>
      </template>
      <div class="search-dialog-body">
        <input
          v-model="chatStore.searchQuery"
          class="search-dialog-input"
          data-testid="search-dialog-input"
          type="text"
          placeholder="Search sessions..."
          autofocus
        />
        <div
          v-if="chatStore.filteredSessions.length === 0"
          class="search-dialog-empty"
        >
          No sessions found
        </div>
        <div v-else class="search-dialog-list">
          <div
            v-for="session in chatStore.filteredSessions"
            :key="session.id"
            class="search-dialog-item"
            :class="{
              'search-dialog-item--active':
                session.id === chatStore.activeSessionId,
            }"
            data-testid="search-dialog-item"
            @click="
              handleSelectSession(session.id);
              closeSearchDialog();
            "
            role="button"
            tabindex="0"
            @keydown.enter="
              handleSelectSession(session.id);
              closeSearchDialog();
            "
          >
            <span class="search-dialog-item-title">{{
              truncateTitle(session.title, 50)
            }}</span>
            <span class="search-dialog-item-meta">
              {{ session.message_count }} msg ·
              {{ formatRelativeTime(session.updated_at) }}
            </span>
          </div>
        </div>
      </div>
    </VDialog>
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
  transition:
    width var(--transition-normal),
    min-width var(--transition-normal),
    padding var(--transition-normal);
}

.session-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.session-header-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.session-header-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  background: var(--color-muted);
  border: 1px solid transparent;
  border-radius: 6px;
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.session-header-btn:hover {
  color: var(--color-foreground);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  border-color: var(--color-primary);
}

.session-title {
  font-size: var(--font-size-xs);
  font-weight: 600;
  color: var(--color-muted-foreground);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.session-new-wrapper {
  display: flex;
  flex-shrink: 0;
}

/* ── New Session Trigger (Pencil Design: Component/NewSessionCollectionControl) ── */
.session-new-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  height: 36px;
  padding: 4px 12px;
  background: var(--color-primary);
  border: 1px solid var(--color-primary);
  border-radius: var(--radius-md);
  color: var(--color-primary-foreground);
  cursor: pointer;
  font-family: var(--font-family);
  transition: all var(--transition-fast);
  outline: none;
  user-select: none;
}

.session-new-trigger:hover {
  opacity: 0.9;
}

.session-new-trigger:active {
  opacity: 0.8;
}

.session-new-trigger:disabled {
  opacity: 0.45;
  cursor: not-allowed;
}

.session-new-label-group {
  display: flex;
  align-items: center;
  gap: 8px;
}

.session-new-plus {
  font-size: 16px;
  font-weight: 700;
  line-height: 1;
}

.session-new-label {
  font-size: 13px;
  font-weight: 600;
  line-height: 1;
}

.session-new-chevron {
  font-size: 12px;
  font-weight: 700;
  line-height: 1;
  transition: transform var(--transition-fast);
}

.session-new-chevron--open {
  transform: rotate(180deg);
}

/* ── New Session Dropdown ── */
.session-new-dropdown {
  position: fixed;
  z-index: 1000;
  background: var(--color-popover);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
  overflow: hidden;
  padding: 6px 0;
}

.session-new-caption {
  padding: 4px 12px 8px;
  font-size: var(--font-size-3xs);
  font-weight: 600;
  color: var(--color-muted-foreground);
  letter-spacing: 0.8px;
  text-transform: uppercase;
}

.session-new-option {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 12px;
  background: transparent;
  border: none;
  color: var(--color-foreground);
  font-family: var(--font-family);
  font-size: 12px;
  cursor: pointer;
  text-align: left;
  outline: none;
  transition: background var(--transition-fast);
  min-height: 32px;
}

.session-new-option:hover {
  background: var(--color-muted);
}

.session-new-option--selected {
  background: rgba(16, 185, 129, 0.15);
}

.session-new-option--selected:hover {
  background: rgba(16, 185, 129, 0.22);
}

.session-new-check {
  font-size: 12px;
  font-weight: 700;
  color: var(--color-muted-foreground);
  width: 14px;
  flex-shrink: 0;
  text-align: center;
}

.session-new-option--selected .session-new-check {
  color: var(--color-primary);
}

.session-new-option-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.session-new-option-count {
  font-size: var(--font-size-2xs);
  color: var(--color-muted-foreground);
  opacity: 0.7;
  flex-shrink: 0;
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

.session-section-header {
  font-family: var(--font-family);
  font-size: 10px;
  font-weight: 600;
  letter-spacing: 1px;
  color: var(--color-muted-foreground);
  text-transform: uppercase;
  padding: var(--space-1) 0 0;
  margin-top: var(--space-1);
  user-select: none;
}

.session-section-header:first-of-type {
  margin-top: 0;
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

.session-item--pinned {
  border-color: var(--color-primary);
}

.session-item-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  opacity: 0;
  transition: opacity var(--transition-fast);
  flex-shrink: 0;
}

.session-item:hover .session-item-actions {
  opacity: 1;
}

.session-action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  background: none;
  border: none;
  border-radius: var(--radius-xs);
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
  padding: 0;
  line-height: 1;
}

.session-action-btn:hover {
  color: var(--color-foreground);
  background: var(--color-border);
}

.session-action-btn--delete:hover {
  color: var(--color-destructive);
  background: color-mix(in srgb, var(--color-destructive) 15%, transparent);
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
  gap: var(--space-3);
}

.toolbar-badges {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  min-width: 0;
}

.toolbar-collection-tag {
  font-size: 11px;
  opacity: 0.75;
}

.toolbar-session-title {
  font-size: var(--font-size-lg);
  font-weight: 700;
  color: var(--color-foreground);
  margin: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  max-width: 420px;
  line-height: 1.3;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.toolbar-icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 8px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all 0.15s;
}

.toolbar-icon-btn:hover {
  background: rgba(128, 128, 128, 0.1);
  color: var(--color-foreground);
}

.toolbar-icon-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
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

.pipeline-status-bar {
  width: 100%;
  max-width: 820px;
  margin: var(--space-3) auto 0;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-3) var(--space-4);
  border: 1px solid color-mix(in srgb, var(--color-primary) 24%, transparent);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
  color: var(--color-muted-foreground);
  font-size: 0.85rem;
}

.pipeline-status-dot {
  width: 8px;
  height: 8px;
  border-radius: 999px;
  background: var(--color-primary);
  box-shadow: 0 0 0 4px color-mix(in srgb, var(--color-primary) 16%, transparent);
  flex-shrink: 0;
}

.pipeline-status-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
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
  border: 1px solid rgba(128, 128, 128, 0.15);
  border-radius: var(--radius-lg);
  background: var(--color-card);
  padding: var(--space-2);
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.06);
  transition:
    border-color var(--transition-fast),
    box-shadow var(--transition-fast);
}

.composer-input-wrap:focus-within {
  border-color: var(--color-primary);
  box-shadow: 0 2px 12px rgba(0, 0, 0, 0.1);
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

/* ── Sidebar Collapse/Expand Button ── */
.sidebar-expand-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  margin: 0 auto;
  background: var(--color-muted);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}

.sidebar-expand-btn:hover {
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
  color: var(--color-foreground);
  border-color: var(--color-primary);
}

/* ── Collapsed Sidebar Search Button ── */
.sidebar-search-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  margin: 0 auto;
  background: var(--color-muted);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}

.sidebar-search-btn:hover {
  border-color: var(--color-primary);
  color: var(--color-foreground);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
}

/* ── Collapsed Sidebar New Session Button ── */
.sidebar-new-session-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 44px;
  height: 44px;
  margin: 0 auto;
  background: var(--color-muted);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg);
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition: all var(--transition-fast);
  flex-shrink: 0;
}

.sidebar-new-session-btn:hover {
  border-color: var(--color-primary);
  color: var(--color-foreground);
  background: color-mix(in srgb, var(--color-primary) 10%, transparent);
}

.sidebar-new-session-icon {
  font-family: "IBM Plex Mono", monospace;
  font-size: 20px;
  font-weight: 500;
  line-height: 1;
}

/* ===== Sidebar Overlay (Mobile) ===== */

.sidebar-overlay {
  display: none;
  position: fixed;
  inset: 0;
  z-index: 80;
  background: rgba(0, 0, 0, 0.5);
}

.session-sidebar--collapsed {
  width: 68px;
  min-width: 68px;
  padding: var(--space-3);
  overflow: hidden;
  align-items: center;
}

.session-sidebar--collapsed .session-header,
.session-sidebar--collapsed .session-new-wrapper,
.session-sidebar--collapsed .session-list,
.session-sidebar--collapsed .session-empty {
  display: none;
}

.rename-dialog-body {
  padding: var(--space-3) 0;
}

.rename-dialog-input {
  width: 100%;
  padding: 8px 12px;
  font-size: var(--font-size-sm);
  font-family: var(--font-family);
  color: var(--color-foreground);
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  outline: none;
  box-sizing: border-box;
  transition: border-color var(--transition-fast);
}

.rename-dialog-input:focus {
  border-color: var(--color-primary);
}

/* ===== Search Dialog ===== */

.search-dialog-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-1) 0;
}

.search-dialog-input {
  width: 100%;
  padding: 10px 14px;
  font-size: var(--font-size-sm);
  font-family: var(--font-family);
  color: var(--color-foreground);
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  outline: none;
  box-sizing: border-box;
  transition: border-color var(--transition-fast);
}

.search-dialog-input:focus {
  border-color: var(--color-primary);
}

.search-dialog-input::placeholder {
  color: var(--color-muted-foreground);
}

.search-dialog-empty {
  text-align: center;
  padding: var(--space-6) 0;
  color: var(--color-muted-foreground);
  font-size: var(--font-size-sm);
}

.search-dialog-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 320px;
  overflow-y: auto;
}

.search-dialog-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: all var(--transition-fast);
}

.search-dialog-item:hover {
  border-color: var(--color-primary);
  opacity: 0.9;
}

.search-dialog-item--active {
  background: var(--color-accent);
  border-color: var(--color-primary);
}

.search-dialog-item-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--color-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.search-dialog-item-meta {
  font-size: 10px;
  color: var(--color-muted-foreground);
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
  .chat-view {
    flex-direction: column;
  }

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

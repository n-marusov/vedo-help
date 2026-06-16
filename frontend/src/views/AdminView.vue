<script setup lang="ts">
import { getApiKey, setApiKey } from '@/api/client';
import CollectionManager from '@/components/CollectionManager.vue';
import DocumentList from '@/components/DocumentList.vue';
import VButton from '@/components/ui/VButton.vue';
import VInput from '@/components/ui/VInput.vue';
import VThemeToggle from '@/components/ui/VThemeToggle.vue';
import { useCollectionStore } from '@/stores/collections';
import { useDocumentStore } from '@/stores/documents';
import { onMounted, ref, watch } from 'vue';

const collectionStore = useCollectionStore();
const documentStore = useDocumentStore();

const apiKeyInput = ref('');
const showAuthInput = ref(true);

onMounted(() => {
  const existingKey = getApiKey();
  if (existingKey) {
    apiKeyInput.value = existingKey;
    showAuthInput.value = false;
    loadData();
  }
});

function loadData() {
  collectionStore.fetchCollections();
  if (collectionStore.activeCollectionId) {
    documentStore.fetchDocuments(collectionStore.activeCollectionId);
  }
}

function handleSetApiKey() {
  const key = apiKeyInput.value.trim();
  if (key) {
    setApiKey(key);
    localStorage.setItem('vedo_api_key', key);
    showAuthInput.value = false;
    loadData();
  }
}

function handleClearApiKey() {
  setApiKey('');
  localStorage.removeItem('vedo_api_key');
  apiKeyInput.value = '';
  showAuthInput.value = true;
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Enter') {
    handleSetApiKey();
  }
}

// Watch for collection changes to reload documents
watch(
  () => collectionStore.activeCollectionId,
  (newId) => {
    if (newId) {
      documentStore.fetchDocuments(newId);
    }
  },
);
</script>

<template>
  <div class="admin-view" data-testid="admin-view">
    <!-- Auth Section -->
    <div v-if="showAuthInput" class="auth-section" data-testid="auth-section">
      <div class="auth-card" data-testid="auth-card">
        <div class="auth-header">
          <h1 class="auth-title">Admin Access</h1>
          <p class="auth-desc">
            Enter your API key to manage collections and documents.
          </p>
        </div>
        <div class="auth-form">
          <VInput
            v-model="apiKeyInput"
            type="password"
            placeholder="Enter API key..."
            @keydown="handleKeydown"
          />
          <VButton variant="primary" @click="handleSetApiKey">Set Key</VButton>
        </div>
        <p class="auth-hint">
          The key is stored locally and can be cleared from the Admin panel.
        </p>
      </div>
    </div>

    <!-- Admin Panel -->
    <div v-else class="admin-panel">
      <!-- Header -->
      <header class="admin-header">
        <div class="admin-header__title-block">
          <h1 class="admin-header__title">Admin Panel</h1>
          <p class="admin-header__subtitle">
            Manage collections, uploads, and indexed knowledge sources
          </p>
        </div>
        <div class="admin-header__actions">
          <VThemeToggle />
          <VButton variant="outline" @click="handleClearApiKey">
            Clear API Key
          </VButton>
        </div>
      </header>

      <!-- Content Panels -->
      <div class="admin-content">
        <!-- Collections Panel -->
        <aside class="collections-panel">
          <CollectionManager />
        </aside>

        <!-- Documents Panel -->
        <main class="documents-panel">
          <DocumentList />
        </main>
      </div>
    </div>
  </div>
</template>

<style scoped>
.admin-view {
  height: 100%;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

/* ═══════════════════════════════════════════════════════════════
   Auth Section
   ═══════════════════════════════════════════════════════════════ */
.auth-section {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: 2rem;
}

.auth-card {
  width: 420px;
  max-width: 100%;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  box-shadow: var(--shadow-lg, 0 10px 40px rgba(0, 0, 0, 0.3));
}

.auth-header {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.auth-title {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-2xl, 24px);
  font-weight: 700;
  color: var(--color-foreground);
}

.auth-desc {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  color: var(--color-muted-foreground);
  line-height: 1.5;
}

.auth-form {
  display: flex;
  flex-direction: row;
  gap: 12px;
  align-items: center;
}

.auth-form .v-input {
  flex: 1;
}

.auth-hint {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-3xs, 10px);
  color: var(--color-muted-foreground);
  opacity: 0.7;
}

/* ═══════════════════════════════════════════════════════════════
   Admin Panel
   ═══════════════════════════════════════════════════════════════ */
.admin-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  padding: 24px;
  gap: 20px;
}

/* ── Header ── */
.admin-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 64px;
  flex-shrink: 0;
}

.admin-header__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.admin-header__title-block {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.admin-header__title {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-2xl, 24px);
  font-weight: 700;
  color: var(--color-foreground);
}

.admin-header__subtitle {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
}

/* ── Content Panels ── */
.admin-content {
  display: flex;
  flex: 1;
  gap: 24px;
  overflow: hidden;
}

/* Collections Panel */
.collections-panel {
  width: 380px;
  min-width: 380px;
  flex-shrink: 0;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: hidden;
}

/* Documents Panel */
.documents-panel {
  flex: 1;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 16px;
  overflow: hidden;
}

/* ═══════════════════════════════════════════════════════════════
   Mobile Responsive
   ═══════════════════════════════════════════════════════════════ */
@media (max-width: 768px) {
  .admin-panel {
    padding: 16px;
    gap: 16px;
  }

  .admin-header {
    height: auto;
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }

  .admin-content {
    flex-direction: column;
  }

  .collections-panel {
    width: 100%;
    min-width: 100%;
    max-height: 280px;
  }
}
</style>

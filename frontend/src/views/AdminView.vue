<script setup lang="ts">
import { getApiKey, setApiKey } from '@/api/client';
import CollectionManager from '@/components/CollectionManager.vue';
import DocumentList from '@/components/DocumentList.vue';
import { useCollectionStore } from '@/stores/collections';
import { useDocumentStore } from '@/stores/documents';
import { onMounted, ref } from 'vue';

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
    // Persist in localStorage
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
import { watch } from 'vue';
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
  <div class="admin-view">
    <!-- Auth section -->
    <div v-if="showAuthInput" class="auth-section">
      <div class="auth-card">
        <h2 class="auth-title">Admin Access</h2>
        <p class="auth-desc">
          Enter your API key to manage collections and documents.
        </p>
        <div class="auth-input-row">
          <input
            v-model="apiKeyInput"
            type="password"
            class="auth-input"
            placeholder="Enter API key..."
            @keydown="handleKeydown"
          />
          <button class="btn-auth" @click="handleSetApiKey">Set Key</button>
        </div>
      </div>
    </div>

    <!-- Admin panel -->
    <div v-else class="admin-panel">
      <div class="panel-header">
        <h2 class="panel-title">Admin Panel</h2>
        <button class="btn-logout" @click="handleClearApiKey">
          Clear API Key
        </button>
      </div>

      <div class="panel-body">
        <!-- Collection sidebar -->
        <aside class="collection-panel">
          <CollectionManager />
        </aside>

        <!-- Document list -->
        <main class="document-panel">
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

/* Auth section */
.auth-section {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 100%;
  padding: 2rem;
}

.auth-card {
  background: #1a1a2e;
  border: 1px solid #2a2a4e;
  border-radius: 12px;
  padding: 2rem;
  width: 400px;
  max-width: 100%;
  box-shadow: 0 10px 40px rgba(0, 0, 0, 0.3);
}

.auth-title {
  margin: 0 0 0.5rem;
  color: #e0e0e0;
  font-size: 1.3rem;
}

.auth-desc {
  margin: 0 0 1.25rem;
  color: #6a6a8a;
  font-size: 0.85rem;
}

.auth-input-row {
  display: flex;
  gap: 0.5rem;
}

.auth-input {
  flex: 1;
  background: #2a2a4e;
  border: 1px solid #3a3a5e;
  border-radius: 8px;
  padding: 0.6rem 0.75rem;
  color: #e0e0e0;
  font-size: 0.9rem;
  font-family: inherit;
}

.auth-input:focus {
  outline: none;
  border-color: #6b9fff;
}

.btn-auth {
  background: #4a6fff;
  border: none;
  border-radius: 8px;
  padding: 0.6rem 1.25rem;
  color: white;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
  white-space: nowrap;
}

.btn-auth:hover {
  background: #5a7fff;
}

/* Admin panel */
.admin-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.panel-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1.5rem;
  border-bottom: 1px solid #2a2a4e;
  background: #1a1a2e;
  flex-shrink: 0;
}

.panel-title {
  margin: 0;
  font-size: 1rem;
  color: #e0e0e0;
}

.btn-logout {
  background: none;
  border: 1px solid #3a3a5e;
  border-radius: 6px;
  padding: 0.35rem 0.75rem;
  color: #8b8bbf;
  font-size: 0.78rem;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-logout:hover {
  background: #2a2a4e;
  color: #ff6b6b;
  border-color: #5a2a2a;
}

.panel-body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.collection-panel {
  width: 300px;
  min-width: 300px;
  background: #16162e;
  border-right: 1px solid #2a2a4e;
  overflow: hidden;
}

.document-panel {
  flex: 1;
  overflow: hidden;
}

/* Mobile responsive */
@media (max-width: 768px) {
  .panel-body {
    flex-direction: column;
  }

  .collection-panel {
    width: 100%;
    min-width: 100%;
    max-height: 250px;
    border-right: none;
    border-bottom: 1px solid #2a2a4e;
  }
}
</style>

<script setup lang="ts">
import CollectionManager from '@/components/CollectionManager.vue';
import DocumentList from '@/components/DocumentList.vue';
import GitRepoManager from '@/components/GitRepoManager.vue';
import { useCollectionStore } from '@/stores/collections';
import { useDocumentStore } from '@/stores/documents';
import { onMounted, ref, watch } from 'vue';

const collectionStore = useCollectionStore();
const documentStore = useDocumentStore();

const activeSourceTab = ref<'documents' | 'git'>('documents');

onMounted(() => {
  loadData();
});

function loadData() {
  collectionStore.fetchCollections();
  if (collectionStore.activeCollectionId) {
    documentStore.fetchDocuments(collectionStore.activeCollectionId);
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
    <div class="admin-panel">
      <div class="admin-content">
        <!-- Collections Panel -->
        <aside class="collections-panel">
          <CollectionManager />
        </aside>

        <!-- Sources Panel -->
        <main class="sources-panel">
          <div class="source-tabs" data-testid="source-tabs">
            <button
              class="source-tab"
              :class="{ 'source-tab--active': activeSourceTab === 'documents' }"
              @click="activeSourceTab = 'documents'"
            >
              Documents
            </button>
            <button
              class="source-tab"
              :class="{ 'source-tab--active': activeSourceTab === 'git' }"
              @click="activeSourceTab = 'git'"
            >
              Git Repositories
            </button>
          </div>

          <DocumentList v-if="activeSourceTab === 'documents'" />
          <GitRepoManager v-else />
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
   Admin Panel
   ═══════════════════════════════════════════════════════════════ */
.admin-panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  padding: 24px;
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

/* Sources Panel */
.sources-panel {
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

/* ── Source Tab Navigation ── */
.source-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.source-tab {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  font-weight: 600;
  padding: 10px 18px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  color: var(--color-muted-foreground);
  cursor: pointer;
  transition:
    color var(--transition-fast),
    border-color var(--transition-fast);
}

.source-tab:hover {
  color: var(--color-foreground);
}

.source-tab--active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
}

/* ═══════════════════════════════════════════════════════════════
   Mobile Responsive
   ═══════════════════════════════════════════════════════════════ */
@media (max-width: 768px) {
  .admin-panel {
    padding: 16px;
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

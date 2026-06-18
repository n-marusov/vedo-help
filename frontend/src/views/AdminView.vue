<script setup lang="ts">
import CollectionManager from '@/components/CollectionManager.vue';
import DocumentList from '@/components/DocumentList.vue';
import GitRepoManager from '@/components/GitRepoManager.vue';
import { useCollectionStore } from '@/stores/collections';
import { useDocumentStore } from '@/stores/documents';
import { onMounted, ref, watch } from 'vue';

const collectionStore = useCollectionStore();
const documentStore = useDocumentStore();

const activeAdminTab = ref<'data' | 'git'>('data');

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
      <!-- Header -->
      <header class="admin-header">
        <div class="admin-header__title-block">
          <h1 class="admin-header__title">Admin Panel</h1>
          <p class="admin-header__subtitle">
            Manage collections, uploads, and indexed knowledge sources
          </p>
        </div>
      </header>

      <!-- Tab Navigation -->
      <div class="admin-tabs" data-testid="admin-tabs">
        <button
          class="admin-tab"
          :class="{ 'admin-tab--active': activeAdminTab === 'data' }"
          @click="activeAdminTab = 'data'"
        >
          Collections & Documents
        </button>
        <button
          class="admin-tab"
          :class="{ 'admin-tab--active': activeAdminTab === 'git' }"
          @click="activeAdminTab = 'git'"
        >
          Git Repositories
        </button>
      </div>

      <!-- Content Panels -->
      <div v-if="activeAdminTab === 'data'" class="admin-content">
        <!-- Collections Panel -->
        <aside class="collections-panel">
          <CollectionManager />
        </aside>

        <!-- Documents Panel -->
        <main class="documents-panel">
          <DocumentList />
        </main>
      </div>

      <!-- Git Repositories Panel -->
      <div v-else class="admin-content">
        <main class="documents-panel">
          <GitRepoManager />
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

/* ── Tab Navigation ── */
.admin-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--color-border);
  flex-shrink: 0;
}

.admin-tab {
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

.admin-tab:hover {
  color: var(--color-foreground);
}

.admin-tab--active {
  color: var(--color-primary);
  border-bottom-color: var(--color-primary);
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

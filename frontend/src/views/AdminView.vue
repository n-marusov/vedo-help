<script setup lang="ts">
import CollectionManager from '@/components/CollectionManager.vue';
import DocumentList from '@/components/DocumentList.vue';
import GitRepoManager from '@/components/GitRepoManager.vue';
import HealthStatus from '@/components/HealthStatus.vue';
import RagPipelineDebug from '@/components/RagPipelineDebug.vue';
import SessionDebug from '@/components/SessionDebug.vue';
import { useCollectionStore } from '@/stores/collections';
import { useDocumentStore } from '@/stores/documents';
import { onMounted, ref, watch } from 'vue';

const collectionStore = useCollectionStore();
const documentStore = useDocumentStore();

const activeTab = ref<'sources' | 'debug' | 'pipeline' | 'health'>('sources');
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
      <!-- Top-level Tab Bar -->
      <div class="admin-tabs" data-testid="admin-tabs">
        <button
          class="admin-tab"
          :class="{ 'admin-tab--active': activeTab === 'sources' }"
          data-testid="admin-tab-sources"
          @click="activeTab = 'sources'"
        >
          Collections &amp; Sources
        </button>
        <button
          class="admin-tab"
          :class="{ 'admin-tab--active': activeTab === 'debug' }"
          data-testid="admin-tab-debug"
          @click="activeTab = 'debug'"
        >
          Session Debug
        </button>
        <button
          class="admin-tab"
          :class="{ 'admin-tab--active': activeTab === 'pipeline' }"
          data-testid="admin-tab-pipeline"
          @click="activeTab = 'pipeline'"
        >
          RAG Pipeline Debug
        </button>
        <button
          class="admin-tab"
          :class="{ 'admin-tab--active': activeTab === 'health' }"
          data-testid="admin-tab-health"
          @click="activeTab = 'health'"
        >
          Service Health
        </button>
      </div>

      <div class="admin-content">
        <!-- Collections Tab Content -->
        <template v-if="activeTab === 'sources'">
          <aside class="collections-panel">
            <CollectionManager />
          </aside>
          <main class="sources-panel">
            <div class="source-tabs" data-testid="source-tabs">
              <button
                class="source-tab"
                :class="{
                  'source-tab--active': activeSourceTab === 'documents',
                }"
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
        </template>

        <!-- Session Debug Tab Content -->
        <template v-if="activeTab === 'debug'">
          <SessionDebug />
        </template>

        <!-- RAG Pipeline Debug Tab Content -->
        <template v-if="activeTab === 'pipeline'">
          <RagPipelineDebug />
        </template>

        <!-- Service Health Tab Content -->
        <template v-if="activeTab === 'health'">
          <HealthStatus />
        </template>
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

/* ── Top-level Admin Tab Navigation ── */
.admin-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--color-border);
  margin-bottom: 24px;
  flex-shrink: 0;
}

.admin-tab {
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  font-weight: 600;
  padding: 12px 24px;
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

/* ── Debug Panel Placeholder (replaced by SessionDebug component) ── */
.debug-panel-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-muted-foreground);
  font-size: var(--font-size-sm, 13px);
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 20px;
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

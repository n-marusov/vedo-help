<script setup lang="ts">
import type { CreateCollectionRequest } from '@/api/types';
import VButton from '@/components/ui/VButton.vue';
import VDialog from '@/components/ui/VDialog.vue';
import VInput from '@/components/ui/VInput.vue';
import { useCollectionStore } from '@/stores/collections';
import { onMounted, ref } from 'vue';

const collectionStore = useCollectionStore();

const showCreateDialog = ref(false);
const showDeleteDialog = ref(false);
const deletingCollection = ref<{
  id: string;
  name: string;
  document_count: number;
} | null>(null);
const newCollection = ref<CreateCollectionRequest>({
  name: '',
  description: '',
});
const isCreating = ref(false);

onMounted(() => {
  collectionStore.fetchCollections();
});

function openCreateDialog() {
  newCollection.value = { name: '', description: '' };
  showCreateDialog.value = true;
}

function closeCreateDialog() {
  showCreateDialog.value = false;
  newCollection.value = { name: '', description: '' };
}

async function handleCreate() {
  if (!newCollection.value.name.trim()) return;
  isCreating.value = true;
  const result = await collectionStore.createCollection({
    name: newCollection.value.name.trim(),
    description: newCollection.value.description?.trim() || undefined,
  });
  isCreating.value = false;
  if (result) {
    closeCreateDialog();
    collectionStore.setActiveCollection(result.id);
  }
}

function promptDelete(col: {
  id: string;
  name: string;
  document_count: number;
}) {
  deletingCollection.value = col;
  showDeleteDialog.value = true;
}

async function handleDeleteConfirm() {
  if (deletingCollection.value) {
    await collectionStore.deleteCollection(deletingCollection.value.id);
    showDeleteDialog.value = false;
    deletingCollection.value = null;
  }
}

function handleSelect(id: string) {
  collectionStore.setActiveCollection(id);
}
</script>

<template>
  <div class="collection-manager">
    <!-- Header -->
    <div class="cm-header">
      <span class="cm-label">COLLECTIONS</span>
      <VButton variant="small" @click="openCreateDialog">+ New</VButton>
    </div>

    <!-- Loading -->
    <div
      v-if="
        collectionStore.isLoading && collectionStore.collections.length === 0
      "
      class="cm-empty"
    >
      <p>Loading...</p>
    </div>

    <!-- Empty -->
    <div v-else-if="collectionStore.collections.length === 0" class="cm-empty">
      <p>No collections yet.</p>
      <p class="cm-empty__hint">Create one to start organizing documents.</p>
    </div>

    <!-- Collection list -->
    <div v-else class="cm-list">
      <button
        v-for="col in collectionStore.collections"
        :key="col.id"
        class="cm-card"
        :class="{
          'cm-card--active': col.id === collectionStore.activeCollectionId,
        }"
        @click="handleSelect(col.id)"
      >
        <div class="cm-card__content">
          <span class="cm-card__name">{{ col.name }}</span>
          <span v-if="col.description" class="cm-card__desc">
            {{ col.description }}
          </span>
          <span class="cm-card__count">
            {{ col.document_count }} document{{
              col.document_count !== 1 ? "s" : ""
            }}
          </span>
        </div>
        <button
          class="cm-card__delete"
          title="Delete collection"
          @click.stop="promptDelete(col)"
        >
          🗑
        </button>
      </button>
    </div>

    <!-- Create Collection Dialog -->
    <VDialog
      :open="showCreateDialog"
      title="Create Collection"
      confirmText="Create"
      cancelText="Cancel"
      @close="closeCreateDialog"
      @confirm="handleCreate"
    >
      <div class="form-group">
        <label class="form-label" for="col-name">Name</label>
        <VInput
          id="col-name"
          v-model="newCollection.name"
          placeholder="e.g., Technical Documentation"
          @keydown="(e: KeyboardEvent) => e.key === 'Enter' && handleCreate()"
        />
      </div>
      <div class="form-group">
        <label class="form-label" for="col-desc">Description (optional)</label>
        <textarea
          id="col-desc"
          v-model="newCollection.description"
          class="form-textarea"
          placeholder="What kind of documents are in this collection?"
          rows="3"
        />
      </div>
    </VDialog>

    <!-- Delete Collection Dialog -->
    <VDialog
      :open="showDeleteDialog"
      title="Delete collection?"
      :description="`This removes the collection and all indexed documents.`"
      confirmText="Delete"
      cancelText="Cancel"
      variant="destructive"
      @close="
        showDeleteDialog = false;
        deletingCollection = null;
      "
      @confirm="handleDeleteConfirm"
    >
      <p class="delete-warning" v-if="deletingCollection">
        <strong>{{ deletingCollection.name }}</strong> contains
        {{ deletingCollection.document_count }} document{{
          deletingCollection.document_count !== 1 ? "s" : ""
        }}. This action cannot be undone.
      </p>
    </VDialog>
  </div>
</template>

<style scoped>
.collection-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

/* ── Header ── */
.cm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.cm-label {
  font-family: var(--font-family);
  font-size: var(--font-size-2xs, 11px);
  font-weight: 600;
  color: var(--color-muted-foreground);
  letter-spacing: 1.5px;
  text-transform: uppercase;
}

/* ── Empty state ── */
.cm-empty {
  padding: 24px 0;
  text-align: center;
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
}

.cm-empty__hint {
  margin-top: 6px;
  font-size: var(--font-size-2xs, 11px);
  color: var(--color-muted-foreground);
  opacity: 0.7;
}

/* ── Collection list ── */
.cm-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.cm-card {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  width: 100%;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 12px);
  padding: 12px;
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition:
    border-color var(--transition-fast, 150ms),
    background var(--transition-fast, 150ms);
}

.cm-card:hover {
  border-color: var(--color-muted-foreground);
}

.cm-card--active {
  border-color: var(--color-primary);
}

.cm-card__content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.cm-card__name {
  font-size: var(--font-size-sm, 13px);
  font-weight: 600;
  color: var(--color-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cm-card__desc {
  font-size: var(--font-size-2xs, 11px);
  color: var(--color-muted-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cm-card__count {
  font-size: var(--font-size-3xs, 10px);
  color: var(--color-muted-foreground);
  opacity: 0.7;
  margin-top: 2px;
}

.cm-card__delete {
  flex-shrink: 0;
  background: none;
  border: none;
  font-size: 14px;
  cursor: pointer;
  padding: 4px;
  border-radius: var(--radius-xs, 4px);
  opacity: 0;
  transition:
    opacity var(--transition-fast, 150ms),
    background var(--transition-fast, 150ms);
  line-height: 1;
}

.cm-card:hover .cm-card__delete {
  opacity: 0.6;
}

.cm-card__delete:hover {
  opacity: 1 !important;
  background: color-mix(in srgb, var(--color-destructive) 15%, transparent);
}

/* ── Form ── */
.form-group {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.form-label {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
  font-weight: 500;
}

.form-textarea {
  width: 100%;
  background: var(--color-secondary);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md, 8px);
  padding: 10px 12px;
  color: var(--color-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  outline: none;
  resize: vertical;
  min-height: 60px;
  transition: border-color var(--transition-fast, 150ms);
  box-sizing: border-box;
}

.form-textarea::placeholder {
  color: var(--color-muted-foreground);
  opacity: 0.6;
}

.form-textarea:focus {
  border-color: var(--color-primary);
}

/* ── Delete warning ── */
.delete-warning {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  color: var(--color-foreground);
  line-height: 1.5;
}
</style>

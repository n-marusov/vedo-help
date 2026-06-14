<script setup lang="ts">
import type { CreateCollectionRequest } from '@/api/types';
import { useCollectionStore } from '@/stores/collections';
import { onMounted, ref } from 'vue';

const collectionStore = useCollectionStore();

const showCreateDialog = ref(false);
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

async function handleDelete(collectionId: string, collectionName: string) {
  if (confirm(`Delete collection "${collectionName}" and all its documents?`)) {
    await collectionStore.deleteCollection(collectionId);
  }
}

function handleSelect(collectionId: string) {
  collectionStore.setActiveCollection(collectionId);
}
</script>

<template>
  <div class="collection-manager">
    <div class="collection-header">
      <h3 class="collection-title">Collections</h3>
      <button class="btn-create" @click="openCreateDialog" title="Create collection">
        + New
      </button>
    </div>

    <div v-if="collectionStore.isLoading && collectionStore.collections.length === 0" class="collection-empty">
      <p>Loading...</p>
    </div>

    <div v-else-if="collectionStore.collections.length === 0" class="collection-empty">
      <p>No collections yet.</p>
      <p class="collection-hint">Create one to start organizing documents.</p>
    </div>

    <div v-else class="collection-list">
      <button
        v-for="col in collectionStore.collections"
        :key="col.id"
        class="collection-item"
        :class="{ active: col.id === collectionStore.activeCollectionId }"
        @click="handleSelect(col.id)"
      >
        <div class="collection-item-content">
          <span class="collection-item-name">{{ col.name }}</span>
          <span class="collection-item-desc" v-if="col.description">
            {{ col.description }}
          </span>
          <span class="collection-item-count">
            {{ col.document_count }} document{{ col.document_count !== 1 ? 's' : '' }}
          </span>
        </div>
        <button
          class="btn-delete-collection"
          @click="handleDelete(col.id, col.name)"
          title="Delete collection"
        >
          🗑️
        </button>
      </button>
    </div>

    <!-- Create dialog overlay -->
    <Teleport to="body">
      <div v-if="showCreateDialog" class="dialog-overlay" @click.self="closeCreateDialog">
        <div class="dialog">
          <div class="dialog-header">
            <h3>Create Collection</h3>
            <button class="dialog-close" @click="closeCreateDialog">×</button>
          </div>
          <div class="dialog-body">
            <div class="form-group">
              <label for="col-name">Name</label>
              <input
                id="col-name"
                v-model="newCollection.name"
                type="text"
                class="form-input"
                placeholder="e.g., Technical Documentation"
                @keydown.enter="handleCreate"
              />
            </div>
            <div class="form-group">
              <label for="col-desc">Description (optional)</label>
              <textarea
                id="col-desc"
                v-model="newCollection.description"
                class="form-textarea"
                placeholder="What kind of documents are in this collection?"
                rows="3"
              />
            </div>
          </div>
          <div class="dialog-footer">
            <button class="btn-cancel" @click="closeCreateDialog">Cancel</button>
            <button
              class="btn-submit"
              :disabled="!newCollection.name.trim() || isCreating"
              @click="handleCreate"
            >
              {{ isCreating ? 'Creating...' : 'Create' }}
            </button>
          </div>
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.collection-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.collection-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #2a2a4e;
  flex-shrink: 0;
}

.collection-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  color: #8b8bbf;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.btn-create {
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

.btn-create:hover {
  background: #5a7fff;
}

.collection-empty {
  padding: 2rem 1rem;
  text-align: center;
  color: #5a5a7a;
  font-size: 0.85rem;
}

.collection-hint {
  margin-top: 0.5rem;
  font-size: 0.78rem;
  color: #4a4a6a;
}

.collection-list {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
}

.collection-item {
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

.collection-item:hover {
  background: #1e1e3e;
  border-color: #2a2a4e;
}

.collection-item.active {
  background: #1a2a4e;
  border-color: #4a6fff;
}

.collection-item-content {
  flex: 1;
  min-width: 0;
}

.collection-item-name {
  display: block;
  font-size: 0.85rem;
  color: #c0c0e0;
  font-weight: 600;
}

.collection-item-desc {
  display: block;
  font-size: 0.72rem;
  color: #6a6a8a;
  margin-top: 0.1rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.collection-item-count {
  display: block;
  font-size: 0.7rem;
  color: #5a5a7a;
  margin-top: 0.15rem;
}

.btn-delete-collection {
  background: none;
  border: none;
  font-size: 0.9rem;
  cursor: pointer;
  padding: 0.25rem;
  border-radius: 4px;
  opacity: 0;
  transition: all 0.2s;
}

.collection-item:hover .btn-delete-collection {
  opacity: 1;
}

.btn-delete-collection:hover {
  background: #3a1a1a;
}

/* Dialog styles */
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog {
  background: #1a1a2e;
  border: 1px solid #2a2a4e;
  border-radius: 12px;
  width: 420px;
  max-width: 90vw;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.5);
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 1rem 1.25rem;
  border-bottom: 1px solid #2a2a4e;
}

.dialog-header h3 {
  margin: 0;
  font-size: 1rem;
  color: #e0e0e0;
}

.dialog-close {
  background: none;
  border: none;
  color: #5a5a7a;
  font-size: 1.4rem;
  cursor: pointer;
  padding: 0;
  line-height: 1;
}

.dialog-close:hover {
  color: #e0e0e0;
}

.dialog-body {
  padding: 1.25rem;
}

.form-group {
  margin-bottom: 1rem;
}

.form-group label {
  display: block;
  font-size: 0.8rem;
  color: #8b8bbf;
  margin-bottom: 0.35rem;
  font-weight: 500;
}

.form-input,
.form-textarea {
  width: 100%;
  background: #2a2a4e;
  border: 1px solid #3a3a5e;
  border-radius: 8px;
  padding: 0.55rem 0.75rem;
  color: #e0e0e0;
  font-size: 0.88rem;
  font-family: inherit;
  box-sizing: border-box;
}

.form-input:focus,
.form-textarea:focus {
  outline: none;
  border-color: #6b9fff;
}

.form-textarea {
  resize: vertical;
  min-height: 60px;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.5rem;
  padding: 1rem 1.25rem;
  border-top: 1px solid #2a2a4e;
}

.btn-cancel {
  background: none;
  border: 1px solid #3a3a5e;
  border-radius: 8px;
  padding: 0.45rem 1rem;
  color: #8b8bbf;
  font-size: 0.85rem;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-cancel:hover {
  background: #2a2a4e;
}

.btn-submit {
  background: #4a6fff;
  border: none;
  border-radius: 8px;
  padding: 0.45rem 1.25rem;
  color: white;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-submit:hover:not(:disabled) {
  background: #5a7fff;
}

.btn-submit:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}
</style>

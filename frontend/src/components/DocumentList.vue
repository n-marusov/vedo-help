<script setup lang="ts">
import { useCollectionStore } from '@/stores/collections';
import { useDocumentStore } from '@/stores/documents';
import { onMounted, ref } from 'vue';

const documentStore = useDocumentStore();
const collectionStore = useCollectionStore();

const fileInput = ref<HTMLInputElement | null>(null);
const uploadProgress = ref<number | null>(null);
const uploadingFileName = ref<string>('');
const isUploading = ref(false);

onMounted(() => {
  loadDocuments();
});

function loadDocuments() {
  if (collectionStore.activeCollectionId) {
    documentStore.fetchDocuments(collectionStore.activeCollectionId);
  }
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${Number.parseFloat((bytes / k ** i).toFixed(1))} ${sizes[i]}`;
}

function formatDate(dateStr: string): string {
  return new Date(dateStr).toLocaleDateString([], {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

function getFileIcon(fileType: string): string {
  const type = fileType.toLowerCase();
  if (type.includes('pdf')) return '📄';
  if (type.includes('markdown') || type.endsWith('md')) return '📝';
  if (type.includes('html')) return '🌐';
  if (type.includes('json')) return '📋';
  if (type.includes('text') || type.endsWith('txt')) return '📃';
  if (type.includes('zip') || type.includes('tar') || type.includes('gz')) return '📦';
  return '📎';
}

function triggerFilePicker() {
  fileInput.value?.click();
}

async function handleFileSelected(event: Event) {
  const input = event.target as HTMLInputElement;
  const files = input.files;
  if (!files || files.length === 0) return;

  const collectionId = collectionStore.activeCollectionId;
  if (!collectionId) {
    alert('Please select a collection first.');
    return;
  }

  isUploading.value = true;

  for (const file of Array.from(files)) {
    uploadingFileName.value = file.name;
    uploadProgress.value = 0;

    await documentStore.uploadDocument(file, collectionId, (progress) => {
      uploadProgress.value = progress;
    });
  }

  isUploading.value = false;
  uploadProgress.value = null;
  uploadingFileName.value = '';
  // Reset file input
  if (fileInput.value) {
    fileInput.value.value = '';
  }
}

async function handleDeleteDocument(docId: string, docName: string) {
  if (confirm(`Delete "${docName}"? This cannot be undone.`)) {
    await documentStore.deleteDocument(docId);
  }
}

// Watch for collection changes
import { watch } from 'vue';
watch(
  () => collectionStore.activeCollectionId,
  () => {
    loadDocuments();
  },
);
</script>

<template>
  <div class="document-list">
    <div class="doc-header">
      <h3 class="doc-title">Documents</h3>
      <div class="doc-actions">
        <input
          ref="fileInput"
          type="file"
          multiple
          accept=".pdf,.md,.txt,.html,.json,.zip"
          class="file-input-hidden"
          @change="handleFileSelected"
        />
        <button
          class="btn-upload"
          :disabled="isUploading || !collectionStore.activeCollectionId"
          @click="triggerFilePicker"
        >
          <span v-if="isUploading">⏳</span>
          <span v-else>📤</span>
          Upload
        </button>
      </div>
    </div>

    <!-- Upload progress -->
    <div v-if="isUploading && uploadProgress !== null" class="upload-progress">
      <div class="upload-info">
        <span class="upload-filename">{{ uploadingFileName }}</span>
        <span class="upload-percent">{{ uploadProgress }}%</span>
      </div>
      <div class="progress-bar">
        <div class="progress-fill" :style="{ width: `${uploadProgress}%` }" />
      </div>
    </div>

    <!-- No collection selected -->
    <div
      v-if="!collectionStore.activeCollectionId"
      class="doc-empty"
    >
      <p>Select a collection to view documents.</p>
    </div>

    <!-- No documents -->
    <div
      v-else-if="documentStore.documents.length === 0 && !documentStore.isLoading"
      class="doc-empty"
    >
      <p>No documents in this collection.</p>
      <p class="doc-hint">Upload PDF, Markdown, TXT, HTML, or ZIP files.</p>
    </div>

    <!-- Loading -->
    <div v-else-if="documentStore.isLoading" class="doc-empty">
      <p>Loading documents...</p>
    </div>

    <!-- Document list -->
    <div v-else class="doc-items">
      <div
        v-for="doc in documentStore.documents"
        :key="doc.id"
        class="doc-item"
      >
        <div class="doc-icon">
          {{ getFileIcon(doc.file_type) }}
        </div>
        <div class="doc-info">
          <span class="doc-name">{{ doc.name }}</span>
          <span class="doc-meta">
            {{ formatFileSize(doc.file_size) }} · {{ formatDate(doc.uploaded_at) }}
          </span>
        </div>
        <button
          class="btn-delete"
          title="Delete document"
          @click="handleDeleteDocument(doc.id, doc.name)"
        >
          🗑️
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.document-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
}

.doc-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  border-bottom: 1px solid #2a2a4e;
  flex-shrink: 0;
}

.doc-title {
  margin: 0;
  font-size: 0.85rem;
  font-weight: 600;
  color: #8b8bbf;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.doc-actions {
  display: flex;
  gap: 0.5rem;
}

.file-input-hidden {
  display: none;
}

.btn-upload {
  display: flex;
  align-items: center;
  gap: 0.3rem;
  background: #4a6fff;
  border: none;
  border-radius: 6px;
  padding: 0.35rem 0.65rem;
  color: white;
  font-size: 0.78rem;
  font-weight: 600;
  cursor: pointer;
  transition: background 0.2s;
}

.btn-upload:hover:not(:disabled) {
  background: #5a7fff;
}

.btn-upload:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.upload-progress {
  padding: 0.5rem 1rem;
  border-bottom: 1px solid #2a2a4e;
  flex-shrink: 0;
}

.upload-info {
  display: flex;
  justify-content: space-between;
  font-size: 0.75rem;
  margin-bottom: 0.3rem;
}

.upload-filename {
  color: #8b8bbf;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.upload-percent {
  color: #4caf50;
  font-weight: 600;
}

.progress-bar {
  height: 4px;
  background: #2a2a4e;
  border-radius: 2px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #4a6fff, #6b9fff);
  border-radius: 2px;
  transition: width 0.3s ease;
}

.doc-empty {
  padding: 2rem 1rem;
  text-align: center;
  color: #5a5a7a;
  font-size: 0.85rem;
}

.doc-hint {
  margin-top: 0.5rem;
  font-size: 0.78rem;
  color: #4a4a6a;
}

.doc-items {
  flex: 1;
  overflow-y: auto;
  padding: 0.5rem;
}

.doc-item {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.6rem 0.75rem;
  border-radius: 8px;
  transition: background 0.2s;
  cursor: default;
}

.doc-item:hover {
  background: #1e1e3e;
}

.doc-icon {
  font-size: 1.2rem;
  flex-shrink: 0;
}

.doc-info {
  flex: 1;
  min-width: 0;
}

.doc-name {
  display: block;
  font-size: 0.82rem;
  color: #c0c0e0;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.doc-meta {
  display: block;
  font-size: 0.7rem;
  color: #5a5a7a;
  margin-top: 0.1rem;
}

.btn-delete {
  background: none;
  border: none;
  font-size: 0.9rem;
  cursor: pointer;
  padding: 0.25rem;
  border-radius: 4px;
  opacity: 0;
  transition: all 0.2s;
}

.doc-item:hover .btn-delete {
  opacity: 1;
}

.btn-delete:hover {
  background: #3a1a1a;
}
</style>

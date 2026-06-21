<script setup lang="ts">
import VButton from '@/components/ui/VButton.vue';
import VDialog from '@/components/ui/VDialog.vue';
import VDropZone from '@/components/ui/VDropZone.vue';
import VProgressBar from '@/components/ui/VProgressBar.vue';
import VSkeleton from '@/components/ui/VSkeleton.vue';
import VToast from '@/components/ui/VToast.vue';
import { useCollectionStore } from '@/stores/collections';
import { useDocumentStore } from '@/stores/documents';
import { computed, onMounted, ref, watch } from 'vue';

const documentStore = useDocumentStore();
const collectionStore = useCollectionStore();

const isUploading = ref(false);
const uploadProgress = ref<number | null>(null);
const uploadingFileName = ref<string>('');

const showDeleteDialog = ref(false);
const deletingDoc = ref<{ id: string; name: string } | null>(null);
const fileInputRef = ref<HTMLInputElement | null>(null);

// ── Bulk delete state ──
const selectedIds = ref<Set<string>>(new Set());
const showBulkDeleteDialog = ref(false);

// ── Toast state ──
const toastMessage = ref('');
const toastType = ref<'info' | 'success' | 'error'>('info');
const showToast = ref(false);

function showToastMessage(message: string, type: 'info' | 'success' | 'error') {
  toastMessage.value = message;
  toastType.value = type;
  showToast.value = true;
}

const isAllSelected = computed(() => {
  const docs = documentStore.documents;
  return docs.length > 0 && docs.every((d) => selectedIds.value.has(d.id));
});

const isSomeSelected = computed(() => {
  return selectedIds.value.size > 0 && !isAllSelected.value;
});

function toggleSelection(id: string) {
  const next = new Set(selectedIds.value);
  if (next.has(id)) {
    next.delete(id);
  } else {
    next.add(id);
  }
  selectedIds.value = next;
}

function toggleAll() {
  if (isAllSelected.value) {
    selectedIds.value = new Set();
  } else {
    selectedIds.value = new Set(documentStore.documents.map((d) => d.id));
  }
}

function promptBulkDelete() {
  if (selectedIds.value.size === 0) {
    showToastMessage('Select documents to delete', 'info');
    return;
  }
  showBulkDeleteDialog.value = true;
}

async function handleBulkDeleteConfirm() {
  if (selectedIds.value.size === 0) return;

  const ids = Array.from(selectedIds.value);
  const count = ids.length;
  const result = await documentStore.deleteDocumentsBatch(ids);

  showBulkDeleteDialog.value = false;

  if (result) {
    selectedIds.value = new Set();
    showToastMessage(`Deleted ${count} document(s)`, 'success');
  } else {
    showToastMessage(documentStore.error || 'Failed to delete documents', 'error');
  }
}

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

function formatFileType(fileType: string): string {
  const type = fileType.toLowerCase();
  if (type.includes('pdf')) return 'PDF';
  if (type.includes('markdown') || type.endsWith('md')) return 'Markdown';
  if (type.includes('html')) return 'HTML';
  if (type.includes('json')) return 'JSON';
  if (type.includes('text') || type.endsWith('txt')) return 'Text';
  if (type.includes('zip') || type.includes('tar') || type.includes('gz')) return 'Archive';
  return type.split('/').pop() || type;
}

const zipResult = ref<{
  processed: number;
  total: number;
  failed: number;
} | null>(null);

async function handleFilesSelected(files: File[]) {
  const collectionId = collectionStore.activeCollectionId;
  if (!collectionId) return;

  isUploading.value = true;
  zipResult.value = null;

  const zipFiles = files.filter((f) => f.name.toLowerCase().endsWith('.zip'));
  const regularFiles = files.filter((f) => !f.name.toLowerCase().endsWith('.zip'));

  for (const file of zipFiles) {
    uploadingFileName.value = file.name;
    uploadProgress.value = 0;

    const result = await documentStore.uploadZip(file, collectionId, (progress) => {
      uploadProgress.value = progress;
    });

    if (result) {
      zipResult.value = {
        processed: result.processed,
        total: result.total_files,
        failed: result.failed,
      };
    }
  }

  for (const file of regularFiles) {
    uploadingFileName.value = file.name;
    uploadProgress.value = 0;

    await documentStore.uploadDocument(file, collectionId, (progress) => {
      uploadProgress.value = progress;
    });
  }

  isUploading.value = false;
  uploadProgress.value = null;
  uploadingFileName.value = '';
}

function triggerFilePick() {
  fileInputRef.value?.click();
}

function handleUploadButtonChange(e: Event) {
  const target = e.target as HTMLInputElement;
  if (target.files && target.files.length > 0) {
    handleFilesSelected(Array.from(target.files));
    target.value = '';
  }
}

function clearZipResult() {
  zipResult.value = null;
}

function promptDelete(doc: { id: string; name: string }) {
  deletingDoc.value = doc;
  showDeleteDialog.value = true;
}

async function handleDeleteConfirm() {
  if (deletingDoc.value) {
    const success = await documentStore.deleteDocument(deletingDoc.value.id);
    if (success) {
      showDeleteDialog.value = false;
      deletingDoc.value = null;
    }
  }
}

watch(
  () => collectionStore.activeCollectionId,
  () => {
    selectedIds.value = new Set();
    loadDocuments();
  },
);
</script>

<template>
  <div class="document-list">
    <!-- ZIP batch result -->
    <div v-if="zipResult" class="dl-zip-result">
      <div class="dl-zip-result__header">
        <span class="dl-zip-result__title">📦 ZIP Batch Result</span>
        <button class="dl-zip-result__close" @click="clearZipResult">✕</button>
      </div>
      <div class="dl-zip-result__summary">
        <strong>{{ zipResult.processed }}</strong> of
        <strong>{{ zipResult.total }}</strong> files processed
        <span v-if="zipResult.failed > 0">
          ·
          <strong style="color: var(--color-destructive)">{{
            zipResult.failed
          }}</strong>
          failed
        </span>
      </div>
    </div>
    <!-- Header -->
    <div class="dl-header">
      <div class="dl-header__left" v-if="collectionStore.activeCollectionId">
        <label class="dl-toggle-all">
          <input
            type="checkbox"
            :checked="isAllSelected"
            :indeterminate="isSomeSelected"
            :disabled="documentStore.documents.length === 0"
            @change="toggleAll"
          />
        </label>
        <span class="dl-label">DOCUMENTS</span>
      </div>
      <span class="dl-label" v-else>DOCUMENTS</span>
      <div class="dl-header__actions">
        <VButton
          v-if="selectedIds.size > 0"
          variant="destructive"
          :disabled="documentStore.isDeleting"
          @click="promptBulkDelete"
        >
          🗑 Delete {{ selectedIds.size }} selected
        </VButton>
        <VButton
          variant="primary"
          :disabled="isUploading || !collectionStore.activeCollectionId"
          @click="triggerFilePick"
        >
          📤 Upload
        </VButton>
        <input
          ref="fileInputRef"
          type="file"
          accept=".pdf,.md,.txt,.html,.json,.zip"
          multiple
          class="dl-file-input"
          @change="handleUploadButtonChange"
        />
      </div>
    </div>

    <!-- No collection selected -->
    <div v-if="!collectionStore.activeCollectionId" class="dl-empty">
      <p>Select a collection to view documents.</p>
    </div>

    <template v-else>
      <!-- Drop zone -->
      <VDropZone
        :disabled="isUploading"
        label="Drop PDF, MD, TXT, HTML, JSON, or ZIP files here"
        @files-selected="handleFilesSelected"
      />

      <!-- Upload progress -->
      <div v-if="isUploading && uploadProgress !== null" class="dl-progress">
        <div class="dl-progress__meta">
          <span class="dl-progress__name">{{ uploadingFileName }}</span>
          <span class="dl-progress__pct">{{ uploadProgress }}%</span>
        </div>
        <VProgressBar :value="uploadProgress" />
      </div>

      <!-- Loading -->
      <div
        v-if="documentStore.isLoading && documentStore.documents.length === 0"
        class="dl-loading"
        data-testid="documents-loading-skeleton"
      >
        <VSkeleton variant="card" :rows="5" />
      </div>

      <!-- No documents -->
      <div
        v-else-if="
          documentStore.documents.length === 0 && !documentStore.isLoading
        "
        class="dl-empty"
      >
        <p>No documents in this collection.</p>
        <p class="dl-empty__hint">Drop files above or use the Upload button.</p>
      </div>

      <!-- Document list -->
      <div v-else class="dl-items">
        <div
          v-for="doc in documentStore.documents"
          :key="doc.id"
          class="dl-item"
          :class="{ 'dl-item--selected': selectedIds.has(doc.id) }"
        >
          <label class="dl-item__checkbox">
            <input
              type="checkbox"
              :checked="selectedIds.has(doc.id)"
              @change="toggleSelection(doc.id)"
            />
          </label>
          <span class="dl-item__icon">{{ getFileIcon(doc.file_type) }}</span>
          <div class="dl-item__info">
            <span class="dl-item__name">{{ doc.name }}</span>
            <span class="dl-item__meta">
              {{ formatFileType(doc.file_type) }},
              {{ formatFileSize(doc.file_size) }} ·
              {{ formatDate(doc.uploaded_at) }}
            </span>
          </div>
          <button
            class="dl-item__delete"
            title="Delete document"
            @click="promptDelete({ id: doc.id, name: doc.name })"
          >
            🗑
          </button>
        </div>
      </div>
    </template>

    <!-- Single Delete Document Dialog -->
    <VDialog
      :open="showDeleteDialog"
      title="Delete document?"
      description="Remove this file from the active collection."
      confirmText="Delete"
      cancelText="Cancel"
      variant="destructive"
      @close="
        showDeleteDialog = false;
        deletingDoc = null;
      "
      @confirm="handleDeleteConfirm"
    >
      <p class="delete-warning" v-if="deletingDoc">
        <strong>{{ deletingDoc.name }}</strong> will be removed from search
        results after deletion.
      </p>
    </VDialog>

    <!-- Bulk Delete Confirmation Dialog -->
    <VDialog
      :open="showBulkDeleteDialog"
      :title="`Delete ${selectedIds.size} documents?`"
      :description="`Remove ${selectedIds.size} files from the active collection.`"
      confirmText="Delete"
      cancelText="Cancel"
      variant="destructive"
      @close="showBulkDeleteDialog = false"
      @confirm="handleBulkDeleteConfirm"
    >
      <p class="delete-warning">
        <strong>{{ selectedIds.size }} document(s)</strong> will be removed from
        search results after deletion.
      </p>
    </VDialog>

    <!-- Toast notification -->
    <VToast
      :show="showToast"
      :message="toastMessage"
      :type="toastType"
      @close="showToast = false"
    />
  </div>
</template>

<style scoped>
.document-list {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  gap: 16px;
}

/* ── Header ── */
.dl-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
  gap: 8px;
}

.dl-header__left {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dl-header__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dl-toggle-all {
  display: flex;
  align-items: center;
  cursor: pointer;
}

.dl-toggle-all input[type="checkbox"],
.dl-item__checkbox input[type="checkbox"] {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: var(--color-primary);
}

.dl-label {
  font-family: var(--font-family);
  font-size: var(--font-size-2xs, 11px);
  font-weight: 600;
  color: var(--color-muted-foreground);
  letter-spacing: 1.5px;
  text-transform: uppercase;
}

/* ── Empty state ── */
.dl-empty {
  padding: 24px 0;
  text-align: center;
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
}

.dl-empty__hint {
  margin-top: 6px;
  font-size: var(--font-size-2xs, 11px);
  opacity: 0.7;
}

/* ── ZIP result ── */
.dl-zip-result {
  padding: 12px;
  border-radius: var(--radius-md, 8px);
  background: var(--color-secondary);
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.dl-zip-result__header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.dl-zip-result__title {
  font-weight: 600;
  color: var(--color-foreground);
}

.dl-zip-result__close {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 14px;
  color: var(--color-muted-foreground);
  padding: 2px 4px;
  border-radius: var(--radius-xs, 4px);
}

.dl-zip-result__close:hover {
  background: var(--color-hover);
}

.dl-zip-result__summary {
  color: var(--color-muted-foreground);
  font-size: var(--font-size-2xs, 11px);
}

.dl-zip-result__summary strong {
  color: var(--color-foreground);
}

/* ── Upload progress ── */
.dl-progress {
  display: flex;
  flex-direction: column;
  gap: 8px;
  flex-shrink: 0;
}

.dl-progress__meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.dl-progress__name {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dl-progress__pct {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
  flex-shrink: 0;
}

/* ── Document list ── */
.dl-items {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.dl-item {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 10px;
  border-radius: var(--radius-md, 8px);
  transition: background var(--transition-fast, 150ms);
  cursor: default;
}

.dl-item--selected {
  background: color-mix(in srgb, var(--color-primary) 8%, transparent);
}

.dl-item:hover {
  background: var(--color-secondary);
}

.dl-item--selected:hover {
  background: color-mix(
    in srgb,
    var(--color-primary) 12%,
    var(--color-secondary)
  );
}

.dl-item__checkbox {
  display: flex;
  align-items: center;
  cursor: pointer;
  flex-shrink: 0;
}

.dl-item__checkbox input[type="checkbox"] {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: var(--color-primary);
}

.dl-item__icon {
  font-size: 16px;
  flex-shrink: 0;
  line-height: 1;
}

.dl-item__info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.dl-item__name {
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
  font-weight: 500;
  color: var(--color-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dl-item__meta {
  font-family: var(--font-family);
  font-size: var(--font-size-3xs, 10px);
  color: var(--color-muted-foreground);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.dl-item__delete {
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

.dl-item:hover .dl-item__delete {
  opacity: 0.6;
}

.dl-item__delete:hover {
  opacity: 1 !important;
  background: color-mix(in srgb, var(--color-destructive) 15%, transparent);
}

/* ── Delete warning ── */
.dl-file-input {
  display: none;
}

.delete-warning {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  color: var(--color-foreground);
  line-height: 1.5;
}
</style>

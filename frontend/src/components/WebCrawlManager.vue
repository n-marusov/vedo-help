<script setup lang="ts">
import { api } from '@/api/client';
import type { CrawlJobSummary, CreateCrawlJobRequest } from '@/api/types';
import VBadge from '@/components/ui/VBadge.vue';
import VButton from '@/components/ui/VButton.vue';
import VDialog from '@/components/ui/VDialog.vue';
import VInput from '@/components/ui/VInput.vue';
import VSkeleton from '@/components/ui/VSkeleton.vue';
import { useCollectionStore } from '@/stores/collections';
import { computed, onMounted, ref } from 'vue';

const jobs = ref<CrawlJobSummary[]>([]);
const collectionStore = useCollectionStore();
const isLoadingJobs = ref(false);

// Form state
const showCreateDialog = ref(false);
const url = ref('');
const depth = ref(2);
const maxPages = ref(50);
const pathPrefix = ref('');
const delayMs = ref(1000);
const isCreating = ref(false);
const urlError = ref<string | null>(null);

// Delete confirmation
const showDeleteDialog = ref(false);
const deletingJob = ref<CrawlJobSummary | null>(null);
const isDeleting = ref(false);

// Cancel
const cancellingJobId = ref<string | null>(null);

const activeCollectionName = computed(
  () =>
    collectionStore.collections.find(
      (collection) => collection.id === collectionStore.activeCollectionId,
    )?.name ?? '',
);

const filteredJobs = computed(() =>
  jobs.value.filter((job) => job.collection_id === collectionStore.activeCollectionId),
);

async function fetchJobs() {
  isLoadingJobs.value = true;
  try {
    jobs.value = await api.listCrawlJobs();
  } catch (err) {
    console.error('[WebCrawlManager] failed to fetch jobs:', err);
  } finally {
    isLoadingJobs.value = false;
  }
}

onMounted(() => {
  fetchJobs();
});

function openCreateDialog() {
  url.value = '';
  depth.value = 2;
  maxPages.value = 50;
  pathPrefix.value = '';
  delayMs.value = 1000;
  urlError.value = null;
  showCreateDialog.value = true;
}

function closeCreateDialog() {
  showCreateDialog.value = false;
  urlError.value = null;
}

function validateUrl(value: string): string | null {
  if (!value.trim()) return 'URL is required';
  if (!value.startsWith('http://') && !value.startsWith('https://')) {
    return 'URL must start with http:// or https://';
  }
  return null;
}

async function handleCreate() {
  urlError.value = validateUrl(url.value);
  if (urlError.value) return;

  const collectionId = collectionStore.activeCollectionId;
  if (!collectionId) return;

  isCreating.value = true;
  try {
    const req: CreateCrawlJobRequest = {
      entry_url: url.value.trim(),
      collection_id: collectionId,
      config: {
        max_depth: depth.value,
        max_pages: maxPages.value,
        delay_ms: delayMs.value,
        path_prefix: pathPrefix.value || undefined,
      },
    };
    await api.createCrawlJob(req);
    closeCreateDialog();
    await fetchJobs();
  } catch (err) {
    console.error('[WebCrawlManager] create job failed:', err);
  } finally {
    isCreating.value = false;
  }
}

async function handleCancel(jobId: string) {
  cancellingJobId.value = jobId;
  try {
    await api.cancelCrawlJob(jobId);
    await fetchJobs();
  } catch (err) {
    console.error('[WebCrawlManager] cancel job failed:', err);
  } finally {
    cancellingJobId.value = null;
  }
}

function promptDelete(job: CrawlJobSummary) {
  deletingJob.value = job;
  showDeleteDialog.value = true;
}

async function handleDeleteConfirm() {
  if (!deletingJob.value) return;
  isDeleting.value = true;
  try {
    await api.deleteCrawlJob(deletingJob.value.id);
    showDeleteDialog.value = false;
    deletingJob.value = null;
    await fetchJobs();
  } catch (err) {
    console.error('[WebCrawlManager] delete job failed:', err);
  } finally {
    isDeleting.value = false;
  }
}

function formatDate(iso: string): string {
  const d = new Date(iso);
  const year = d.getFullYear();
  const month = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  const hour = String(d.getHours()).padStart(2, '0');
  const minute = String(d.getMinutes()).padStart(2, '0');
  return `${year}-${month}-${day} ${hour}:${minute}`;
}

function statusBadgeVariant(status: string): 'default' | 'success' | 'warning' | 'info' {
  switch (status) {
    case 'idle':
      return 'default';
    case 'crawling':
      return 'info';
    case 'completed':
      return 'success';
    case 'cancelled':
      return 'default';
    case 'error':
      return 'warning';
    default:
      return 'default';
  }
}

const canStartCrawl = computed(() => !!collectionStore.activeCollectionId);
</script>

<template>
  <div class="web-crawl-manager" data-testid="web-crawl-manager">
    <!-- Header -->
    <div class="wcm-header">
      <span class="wcm-label">WEB CRAWL JOBS</span>
      <VButton
        variant="small"
        :disabled="!canStartCrawl"
        @click="openCreateDialog"
      >
        + New Crawl
      </VButton>
    </div>

    <!-- No active collection -->
    <div v-if="!collectionStore.activeCollectionId" class="wcm-empty">
      <p>Select a collection to manage crawl jobs.</p>
    </div>

    <!-- Loading state -->
    <div v-else-if="isLoadingJobs" class="wcm-loading">
      <VSkeleton variant="card" :rows="3" />
    </div>

    <!-- Empty state -->
    <div
      v-else-if="filteredJobs.length === 0"
      class="wcm-empty"
      data-testid="web-crawl-empty-state"
    >
      <p>No crawl jobs yet. Start a new crawl to index website content.</p>
    </div>

    <!-- Job list -->
    <div v-else class="wcm-table-wrapper">
      <table class="wcm-table">
        <thead>
          <tr>
            <th>URL</th>
            <th>Status</th>
            <th>Pages</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="job in filteredJobs" :key="job.id">
            <td class="wcm-cell-url" :title="job.entry_url">
              {{ job.entry_url }}
            </td>
            <td>
              <VBadge :variant="statusBadgeVariant(job.status)">
                {{ job.status }}
              </VBadge>
            </td>
            <td>{{ job.pages_indexed }} / {{ job.pages_found }}</td>
            <td class="wcm-cell-date">{{ formatDate(job.created_at) }}</td>
            <td>
              <div class="wcm-actions">
                <VButton
                  variant="small"
                  :disabled="
                    job.status === 'cancelled' ||
                    job.status === 'completed' ||
                    cancellingJobId === job.id
                  "
                  @click="handleCancel(job.id)"
                >
                  Cancel
                </VButton>
                <VButton variant="ghost" @click="promptDelete(job)">
                  Delete
                </VButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Create dialog -->
    <VDialog :open="showCreateDialog" title="New Web Crawl" @close="closeCreateDialog">
      <div class="wcm-form">
        <VInput
          v-model="url"
          label="Entry URL"
          placeholder="https://example.com/docs"
          data-testid="web-crawl-url-input"
        />
        <p
          v-if="urlError"
          class="wcm-form-error"
          data-testid="web-crawl-url-error"
        >
          {{ urlError }}
        </p>

        <div class="wcm-form-row">
          <label class="wcm-slider-label">
            Max Depth: {{ depth }}
            <input
              v-model.number="depth"
              type="range"
              min="1"
              max="10"
              step="1"
              class="wcm-slider"
              data-testid="web-crawl-depth-slider"
            />
          </label>
        </div>

        <div class="wcm-form-row">
          <label class="wcm-slider-label">
            Max Pages:
            <input
              v-model.number="maxPages"
              type="number"
              min="1"
              max="10000"
              class="wcm-number-input"
              data-testid="web-crawl-max-pages-input"
            />
          </label>
        </div>

        <VInput
          v-model="pathPrefix"
          label="Path Prefix (optional)"
          placeholder="/docs"
          data-testid="web-crawl-path-prefix-input"
        />

        <div class="wcm-form-row">
          <label class="wcm-slider-label">
            Delay: {{ delayMs }}ms
            <input
              v-model.number="delayMs"
              type="range"
              min="100"
              max="5000"
              step="100"
              class="wcm-slider"
              data-testid="web-crawl-delay-slider"
            />
          </label>
        </div>

        <p v-if="activeCollectionName" class="wcm-form-hint">
          Collection: <strong>{{ activeCollectionName }}</strong>
        </p>
      </div>
      <template #actions>
        <VButton variant="ghost" @click="closeCreateDialog">Cancel</VButton>
        <VButton
          variant="primary"
          :disabled="isCreating"
          data-testid="btn-web-crawl-start"
          @click="handleCreate"
        >
          Start Crawl
        </VButton>
      </template>
    </VDialog>

    <!-- Delete confirmation dialog -->
    <VDialog
      :open="showDeleteDialog"
      title="Delete Crawl Job"
      @close="showDeleteDialog = false"
    >
      <p>
        Are you sure you want to delete this crawl job?
        <strong>{{ deletingJob?.entry_url }}</strong>
      </p>
      <template #actions>
        <VButton variant="ghost" @click="showDeleteDialog = false">
          Cancel
        </VButton>
        <VButton variant="destructive" :disabled="isDeleting" @click="handleDeleteConfirm">
          Delete
        </VButton>
      </template>
    </VDialog>
  </div>
</template>

<style scoped>
.web-crawl-manager {
  display: flex;
  flex-direction: column;
  gap: 16px;
  height: 100%;
}

.wcm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.wcm-label {
  font-size: var(--font-size-xs, 11px);
  font-weight: 700;
  letter-spacing: 0.08em;
  color: var(--color-muted-foreground);
}

.wcm-empty {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--color-muted-foreground);
  font-size: var(--font-size-sm, 13px);
  text-align: center;
  padding: 20px;
}

.wcm-loading {
  padding: 8px 0;
}

.wcm-table-wrapper {
  flex: 1;
  overflow-y: auto;
}

.wcm-table {
  width: 100%;
  border-collapse: collapse;
  font-size: var(--font-size-sm, 13px);
}

.wcm-table th {
  text-align: left;
  font-weight: 600;
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  position: sticky;
  top: 0;
  background: var(--color-card);
}

.wcm-table td {
  padding: 10px 12px;
  border-bottom: 1px solid var(--color-border);
  vertical-align: middle;
}

.wcm-table tbody tr:hover {
  background: var(--color-accent);
}

.wcm-cell-url {
  max-width: 240px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.wcm-cell-date {
  white-space: nowrap;
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
}

.wcm-actions {
  display: flex;
  gap: 6px;
}

.wcm-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.wcm-form-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.wcm-slider-label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: var(--font-size-sm, 13px);
  font-weight: 500;
  color: var(--color-foreground);
}

.wcm-slider {
  width: 100%;
  accent-color: var(--color-primary);
}

.wcm-form-error {
  color: var(--color-destructive);
  font-size: var(--font-size-xs, 12px);
  margin: -8px 0 0;
}

.wcm-form-hint {
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
  margin: 0;
}
</style>

<script setup lang="ts">
import { api } from '@/api/client';
import type { CreateRepoRequest, GitRepoSummary } from '@/api/types';
import VBadge from '@/components/ui/VBadge.vue';
import VButton from '@/components/ui/VButton.vue';
import VDialog from '@/components/ui/VDialog.vue';
import VInput from '@/components/ui/VInput.vue';
import { useCollectionStore } from '@/stores/collections';
import { computed, onMounted, ref } from 'vue';
const repos = ref<GitRepoSummary[]>([]);
const collectionStore = useCollectionStore();
const isLoadingRepos = ref(false);

type ConnectRepoForm = Omit<CreateRepoRequest, 'branch' | 'access_token' | 'collection_id'> & {
  branch: string;
  access_token: string;
};

// Connect dialog
const showConnectDialog = ref(false);
const connectForm = ref<ConnectRepoForm>({
  url: '',
  branch: 'main',
  access_token: '',
});
const isConnecting = ref(false);
const connectError = ref<string | null>(null);

// Delete confirmation
const showDeleteDialog = ref(false);
const deletingRepo = ref<GitRepoSummary | null>(null);
const isDeleting = ref(false);

// Tooltip for error status
const hoveredRepoId = ref<string | null>(null);

const activeCollectionName = computed(
  () =>
    collectionStore.collections.find(
      (collection) => collection.id === collectionStore.activeCollectionId,
    )?.name ?? '',
);

const filteredRepos = computed(() =>
  repos.value.filter((repo) => repo.collection_id === collectionStore.activeCollectionId),
);

// ── Lifecycle ──
onMounted(() => {
  fetchRepos();
});

// ── Data fetching ──
async function fetchRepos() {
  isLoadingRepos.value = true;
  console.debug('[GitRepoManager] fetching repos...');
  try {
    repos.value = await api.getGitRepos();
  } catch (err) {
    console.error('[GitRepoManager] failed to fetch repos:', err);
  } finally {
    isLoadingRepos.value = false;
  }
}

// ── Connect repo ──
function openConnectDialog() {
  connectForm.value = {
    url: '',
    branch: 'main',
    access_token: '',
  };
  connectError.value = null;
  showConnectDialog.value = true;
}

function closeConnectDialog() {
  showConnectDialog.value = false;
  connectError.value = null;
}

function validateUrl(url: string): boolean {
  return url.startsWith('https://') || url.startsWith('git@');
}

async function handleConnect() {
  const form = connectForm.value;

  const collectionId = collectionStore.activeCollectionId;
  if (!collectionId) {
    connectError.value = 'Select a collection before connecting a repository.';
    return;
  }

  // Validate URL
  if (!form.url.trim()) {
    connectError.value = 'Repository URL is required.';
    return;
  }
  if (!validateUrl(form.url.trim())) {
    connectError.value = 'URL must start with https:// or git@';
    return;
  }

  isConnecting.value = true;
  connectError.value = null;

  try {
    const repo = await api.createGitRepo({
      url: form.url.trim(),
      branch: form.branch || 'main',
      access_token: form.access_token || undefined,
      collection_id: collectionId,
    });
    repos.value.push(repo);
    closeConnectDialog();
    console.debug('[GitRepoManager] repo connected:', repo.id);
  } catch (err) {
    console.error('[GitRepoManager] connect failed:', err);
    connectError.value = err instanceof Error ? err.message : 'Failed to connect repository.';
  } finally {
    isConnecting.value = false;
  }
}

// ── Sync repo ──
async function syncRepo(repo: GitRepoSummary) {
  // Optimistically set local status
  const idx = repos.value.findIndex((r) => r.id === repo.id);
  if (idx === -1) return;
  repos.value[idx] = { ...repos.value[idx], status: 'syncing' };

  console.debug('[GitRepoManager] triggering sync for repo:', repo.id);
  try {
    const result = await api.triggerSync(repo.id);
    // Update with response data
    repos.value[idx] = {
      ...repos.value[idx],
      status: (result.status as GitRepoSummary['status']) || 'idle',
      last_commit_hash: result.last_commit,
      last_synced_at: new Date().toISOString(),
    };
    console.debug('[GitRepoManager] sync result:', result);
  } catch (err) {
    console.error('[GitRepoManager] sync failed:', err);
    repos.value[idx] = { ...repos.value[idx], status: 'error' };
  }
}

// ── Delete repo ──
function promptDelete(repo: GitRepoSummary) {
  deletingRepo.value = repo;
  showDeleteDialog.value = true;
}

async function handleDeleteConfirm() {
  if (!deletingRepo.value) return;

  isDeleting.value = true;
  console.debug('[GitRepoManager] deleting repo:', deletingRepo.value.id);
  try {
    await api.deleteGitRepo(deletingRepo.value.id);
    repos.value = repos.value.filter((r) => r.id !== deletingRepo.value?.id);
    showDeleteDialog.value = false;
    deletingRepo.value = null;
  } catch (err) {
    console.error('[GitRepoManager] delete failed:', err);
  } finally {
    isDeleting.value = false;
  }
}

// ── Helpers ──
function formatDate(iso?: string): string {
  if (!iso) return '—';
  return new Date(iso).toLocaleDateString([], {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}
</script>

<template>
  <div class="git-repo-manager" data-testid="git-repo-manager">
    <!-- Header -->
    <div class="grm-header">
      <span class="grm-label">GIT REPOSITORIES</span>
      <VButton
        variant="primary"
        data-testid="btn-git-repo-connect"
        :disabled="!collectionStore.activeCollectionId"
        @click="openConnectDialog"
      >
        Connect Repository
      </VButton>
    </div>

    <!-- No collection selected -->
    <div v-if="!collectionStore.activeCollectionId" class="grm-empty">
      <p>Select a collection to view Git repositories.</p>
    </div>

    <!-- Loading -->
    <div v-else-if="isLoadingRepos" class="grm-empty">
      <p>Loading repositories...</p>
    </div>

    <!-- Empty state -->
    <div
      v-else-if="filteredRepos.length === 0"
      class="grm-empty"
      data-testid="git-repo-empty-state"
    >
      <p>
        No repositories connected to this collection. Connect a Git repository
        to index its documentation.
      </p>
    </div>

    <!-- Repo table -->
    <div v-else class="grm-table-wrapper">
      <table class="grm-table">
        <thead>
          <tr>
            <th>URL</th>
            <th>Branch</th>
            <th>Status</th>
            <th>Last Synced</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="repo in filteredRepos"
            :key="repo.id"
            data-testid="git-repo-row"
          >
            <td class="grm-cell-url" :title="repo.url">{{ repo.url }}</td>
            <td>{{ repo.branch }}</td>

            <td>
              <div class="grm-status-cell">
                <VBadge
                  data-testid="git-repo-status"
                  :variant="
                    repo.status === 'syncing'
                      ? 'info'
                      : repo.status === 'error'
                        ? 'default'
                        : 'default'
                  "
                  :class="{
                    'grm-badge-syncing': repo.status === 'syncing',
                    'grm-badge-error': repo.status === 'error',
                    'grm-badge-idle': repo.status === 'idle',
                  }"
                  @mouseenter="
                    repo.status === 'error' ? (hoveredRepoId = repo.id) : null
                  "
                  @mouseleave="hoveredRepoId = null"
                >
                  <span v-if="repo.status === 'syncing'" class="grm-sync-icon"
                    >⟳</span
                  >
                  {{ repo.status }}
                </VBadge>
                <div
                  v-if="repo.status === 'error' && hoveredRepoId === repo.id"
                  class="grm-error-tooltip"
                >
                  Sync failed. Check logs for details.
                </div>
              </div>
            </td>
            <td>{{ formatDate(repo.last_synced_at) }}</td>
            <td>
              <div class="grm-actions">
                <VButton
                  variant="ghost"
                  data-testid="btn-git-sync-now"
                  :disabled="repo.status === 'syncing'"
                  @click="syncRepo(repo)"
                >
                  Sync Now
                </VButton>
                <VButton
                  variant="destructive"
                  data-testid="btn-git-repo-delete"
                  @click="promptDelete(repo)"
                >
                  Delete
                </VButton>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <!-- Connect Repository Dialog -->
    <VDialog
      :open="showConnectDialog"
      title="Connect Repository"
      @close="closeConnectDialog"
    >
      <div class="grm-form">
        <VInput
          v-model="connectForm.url"
          data-testid="git-repo-url-input"
          placeholder="https://github.com/user/repo.git"
          type="text"
        />
        <VInput
          v-model="connectForm.branch"
          data-testid="git-repo-branch-input"
          placeholder="main"
          type="text"
        />
        <VInput
          v-model="connectForm.access_token"
          data-testid="git-repo-token-input"
          placeholder="ghp_... or glpat-..."
          type="password"
        />
        <p v-if="activeCollectionName" class="grm-form-hint">
          Repository will be connected to
          <strong>{{ activeCollectionName }}</strong
          >.
        </p>
        <p
          v-else
          class="grm-form-hint"
          data-testid="git-repo-no-collections-hint"
        >
          Select or create a collection first.
        </p>
        <p
          v-if="connectError"
          class="grm-form-error"
          data-testid="git-repo-url-error"
        >
          {{ connectError }}
        </p>
      </div>
      <template #actions>
        <VButton
          variant="outline"
          data-testid="btn-confirm-cancel"
          @click="closeConnectDialog"
          >Cancel</VButton
        >
        <VButton
          variant="primary"
          data-testid="btn-git-repo-register"
          :disabled="isConnecting"
          @click="handleConnect"
        >
          {{ isConnecting ? "Connecting..." : "Connect" }}
        </VButton>
      </template>
    </VDialog>

    <!-- Delete Confirmation Dialog -->
    <VDialog
      :open="showDeleteDialog"
      title="Delete repository?"
      :description="`Remove ${deletingRepo?.url || ''} and its indexed data.`"
      confirmText="Delete"
      cancelText="Cancel"
      variant="destructive"
      @close="
        showDeleteDialog = false;
        deletingRepo = null;
      "
      @confirm="handleDeleteConfirm"
    />
  </div>
</template>

<style scoped>
.git-repo-manager {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  gap: 16px;
}

/* ── Header ── */
.grm-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  flex-shrink: 0;
}

.grm-label {
  font-family: var(--font-family);
  font-size: var(--font-size-2xs, 11px);
  font-weight: 600;
  color: var(--color-muted-foreground);
  letter-spacing: 1.5px;
  text-transform: uppercase;
}

/* ── Empty state ── */
.grm-empty {
  padding: 24px 0;
  text-align: center;
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
}

/* ── Table ── */
.grm-table-wrapper {
  flex: 1;
  overflow: auto;
}

.grm-table {
  width: 100%;
  border-collapse: collapse;
  font-family: var(--font-family);
  font-size: var(--font-size-xs, 12px);
}

.grm-table th {
  text-align: left;
  padding: 8px 10px;
  font-weight: 600;
  color: var(--color-muted-foreground);
  font-size: var(--font-size-2xs, 11px);
  letter-spacing: 0.5px;
  text-transform: uppercase;
  border-bottom: 1px solid var(--color-border);
  white-space: nowrap;
}

.grm-table td {
  padding: 10px;
  color: var(--color-foreground);
  border-bottom: 1px solid var(--color-border);
  vertical-align: middle;
}

.grm-table tbody tr:hover {
  background: var(--color-secondary);
}

.grm-cell-url {
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* ── Status cell ── */
.grm-status-cell {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.grm-badge-idle {
  background: var(--color-secondary);
  color: var(--color-muted-foreground);
}

.grm-badge-syncing {
  background: color-mix(in srgb, var(--color-info, #3b82f6) 20%, transparent);
  color: var(--color-info, #3b82f6);
  animation: grm-pulse 1.5s ease-in-out infinite;
}

.grm-badge-error {
  background: color-mix(in srgb, var(--color-destructive) 20%, transparent);
  color: var(--color-destructive);
}

@keyframes grm-pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.6;
  }
}

.grm-sync-icon {
  display: inline-block;
  animation: grm-spin 1s linear infinite;
}

@keyframes grm-spin {
  from {
    transform: rotate(0deg);
  }
  to {
    transform: rotate(360deg);
  }
}

/* ── Error tooltip ── */
.grm-error-tooltip {
  position: absolute;
  top: 100%;
  left: 0;
  margin-top: 4px;
  padding: 6px 10px;
  background: var(--color-popover);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm, 6px);
  font-size: var(--font-size-3xs, 10px);
  color: var(--color-destructive);
  white-space: nowrap;
  z-index: 100;
  box-shadow: var(--shadow-md);
}

/* ── Actions ── */
.grm-actions {
  display: flex;
  gap: 6px;
  align-items: center;
}

/* ── Connect form ── */
.grm-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.grm-form-error {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-2xs, 11px);
  color: var(--color-destructive);
}

.grm-form-hint {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-2xs, 11px);
  color: var(--color-muted-foreground);
  line-height: 1.4;
}
</style>

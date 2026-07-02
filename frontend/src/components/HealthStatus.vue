<script setup lang="ts">
import { api } from '@/api/client';
import type { HealthReport } from '@/api/types';
import { onMounted, ref } from 'vue';

const health = ref<HealthReport | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

async function fetchHealth() {
  loading.value = true;
  error.value = null;
  try {
    health.value = await api.getHealthStatus();
  } catch (e) {
    error.value = e instanceof Error ? e.message : 'Unknown error';
    health.value = null;
  } finally {
    loading.value = false;
  }
}

function statusDot(status: string): string {
  switch (status) {
    case 'healthy':
      return 'var(--color-success, #22c55e)';
    case 'degraded':
      return 'var(--color-warning, #eab308)';
    case 'unhealthy':
      return 'var(--color-error, #ef4444)';
    default:
      return 'var(--color-muted-foreground, #6b7280)';
  }
}

function statusLabel(status: string): string {
  switch (status) {
    case 'healthy':
      return 'Healthy';
    case 'degraded':
      return 'Degraded';
    case 'unhealthy':
      return 'Unhealthy';
    default:
      return 'Unknown';
  }
}

function checksHealthy(): number {
  return health.value?.checks.filter((c) => c.status === 'healthy').length ?? 0;
}

function checksUnhealthy(): number {
  return health.value?.checks.filter((c) => c.status !== 'healthy').length ?? 0;
}

onMounted(fetchHealth);
</script>

<template>
  <div class="health-status" data-testid="health-status">
    <!-- Summary header -->
    <div class="health-header" :class="{ 'health-header--loading': loading }">
      <template v-if="loading">
        <span
          class="health-dot"
          :style="{ backgroundColor: 'var(--color-muted-foreground)' }"
        />
        <span class="health-text">Checking...</span>
      </template>
      <template v-else-if="error">
        <span
          class="health-dot"
          :style="{ backgroundColor: 'var(--color-muted-foreground)' }"
        />
        <span class="health-text">Unknown</span>
      </template>
      <template v-else-if="health">
        <span
          class="health-dot"
          :style="{ backgroundColor: statusDot(health.status) }"
        />
        <span class="health-text">{{ statusLabel(health.status) }}</span>
        <span class="health-counts">
          {{ checksHealthy() }}/{{ health.checks.length }} up
        </span>
        <span v-if="checksUnhealthy() > 0" class="health-badge">
          {{ checksUnhealthy() }} down
        </span>
      </template>
      <template v-else>
        <span
          class="health-dot"
          :style="{ backgroundColor: 'var(--color-muted-foreground)' }"
        />
        <span class="health-text">Unknown</span>
      </template>
      <span class="health-refresh-btn" title="Refresh" @click="fetchHealth()"
        >↻</span
      >
    </div>

    <!-- Per-service table (always visible) -->
    <div v-if="health" class="health-detail" data-testid="health-detail">
      <table class="health-table">
        <thead>
          <tr>
            <th>Service</th>
            <th>Status</th>
            <th>Latency</th>
            <th>Error</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="check in health.checks"
            :key="check.name"
            :class="{ 'health-row--unhealthy': check.status !== 'healthy' }"
          >
            <td class="health-cell-name">{{ check.name }}</td>
            <td>
              <span
                class="health-dot health-dot--small"
                :style="{ backgroundColor: statusDot(check.status) }"
              />
              {{ statusLabel(check.status) }}
            </td>
            <td class="health-cell-latency">{{ check.latency_ms }}ms</td>
            <td class="health-cell-error">{{ check.error ?? "—" }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.health-status {
  font-family: var(--font-family, ui-sans-serif, system-ui, sans-serif);
  font-size: var(--font-size-xs, 12px);
  padding: 20px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
}

.health-header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 12px;
  border-radius: var(--radius-lg, 8px);
  color: var(--color-foreground);
  margin-bottom: 12px;
}

.health-header--loading {
  opacity: 0.7;
}

.health-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
  display: inline-block;
}

.health-dot--small {
  width: 8px;
  height: 8px;
}

.health-text {
  font-weight: 600;
}

.health-counts {
  color: var(--color-muted-foreground);
  margin-left: auto;
}

.health-badge {
  background: var(--color-error, #ef4444);
  color: white;
  font-size: 10px;
  font-weight: 700;
  padding: 1px 6px;
  border-radius: 10px;
}

.health-refresh-btn {
  cursor: pointer;
  opacity: 0.6;
  font-size: 14px;
  padding: 0 2px;
  margin-left: 8px;
}

.health-refresh-btn:hover {
  opacity: 1;
}

.health-detail {
  background: var(--color-background);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-lg, 8px);
  overflow: hidden;
}

.health-table {
  width: 100%;
  border-collapse: collapse;
}

.health-table th {
  text-align: left;
  padding: 8px 12px;
  font-weight: 600;
  font-size: 11px;
  text-transform: uppercase;
  color: var(--color-muted-foreground);
  border-bottom: 1px solid var(--color-border);
  background: var(--color-card);
}

.health-table td {
  padding: 8px 12px;
  border-bottom: 1px solid var(--color-border);
  vertical-align: middle;
}

.health-table tr:last-child td {
  border-bottom: none;
}

.health-row--unhealthy {
  background: rgba(239, 68, 68, 0.05);
}

.health-cell-name {
  font-weight: 600;
}

.health-cell-latency {
  color: var(--color-muted-foreground);
  font-variant-numeric: tabular-nums;
}

.health-cell-error {
  color: var(--color-error, #ef4444);
  font-size: 11px;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>

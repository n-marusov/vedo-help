<script setup lang="ts">
import type { Collection } from '@/api/types';
import VBadge from '@/components/ui/VBadge.vue';
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';

const props = withDefaults(
  defineProps<{
    modelValue: string | null | undefined;
    collections: Collection[];
    activeCollectionId: string | null | undefined;
    placeholder?: string;
  }>(),
  {
    placeholder: 'Select a collection...',
  },
);

const emit = defineEmits<{
  'update:modelValue': [value: string | null];
}>();

const open = ref(false);
const triggerRef = ref<HTMLElement | null>(null);
const dropdownRef = ref<HTMLElement | null>(null);
const dropdownStyle = ref<Record<string, string>>({});

const selectedName = computed(() => {
  if (!props.modelValue) return null;
  const col = props.collections.find((c) => c.id === props.modelValue);
  return col?.name || null;
});

const hasSelection = computed(() => !!props.modelValue);

function toggle() {
  open.value = !open.value;
  if (open.value) {
    nextTick(() => updateDropdownPosition());
  }
}

function updateDropdownPosition() {
  if (!triggerRef.value) return;
  const rect = triggerRef.value.getBoundingClientRect();
  dropdownStyle.value = {
    left: `${rect.left}px`,
    minWidth: `${rect.width}px`,
    top: `${rect.bottom + 4}px`,
  };
}

function select(value: string) {
  if (value === props.modelValue) {
    emit('update:modelValue', null);
  } else {
    emit('update:modelValue', value);
  }
  open.value = false;
}

function clearSelection(e: MouseEvent) {
  e.stopPropagation();
  emit('update:modelValue', null);
}

function handleClickOutside(e: MouseEvent) {
  if (
    triggerRef.value &&
    !triggerRef.value.contains(e.target as Node) &&
    dropdownRef.value &&
    !dropdownRef.value.contains(e.target as Node)
  ) {
    open.value = false;
  }
}

function handleKeydown(e: KeyboardEvent) {
  if (e.key === 'Escape') {
    open.value = false;
  }
}

onMounted(() => {
  document.addEventListener('click', handleClickOutside);
  document.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  document.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <div class="collection-selector">
    <!-- Tag/badge when collection is selected -->
    <div
      v-if="hasSelection"
      class="cs-tag"
      data-testid="collection-selector-tag"
      @click="toggle"
      role="button"
      tabindex="0"
      @keydown.enter="toggle"
    >
      <VBadge size="sm" variant="info">{{ selectedName }}</VBadge>
      <button
        class="cs-clear"
        title="Clear selection"
        @click.stop="clearSelection"
      >
        <svg
          fill="none"
          height="12"
          viewBox="0 0 12 12"
          width="12"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d="M3 3L9 9M9 3L3 9"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
          />
        </svg>
      </button>
      <span class="cs-chevron">▾</span>
    </div>

    <!-- Trigger button when no collection is selected -->
    <button
      v-else
      ref="triggerRef"
      class="cs-trigger"
      data-testid="collection-selector-trigger"
      type="button"
      @click="toggle"
    >
      <span class="cs-placeholder">{{ placeholder }}</span>
      <span class="cs-chevron">▾</span>
    </button>

    <Teleport to="body">
      <div
        v-if="open"
        ref="dropdownRef"
        class="cs-dropdown"
        data-testid="collection-selector-dropdown"
        :style="dropdownStyle"
      >
        <button
          v-for="col in collections"
          :key="col.id"
          class="cs-option"
          :class="{ 'cs-option--selected': col.id === modelValue }"
          type="button"
          @click="select(col.id)"
        >
          <span class="cs-option-label">{{ col.name }}</span>
          <span v-if="col.document_count !== undefined" class="cs-option-count">
            {{ col.document_count }}
          </span>
          <span v-if="col.id === modelValue" class="cs-check">✓</span>
        </button>
        <div v-if="collections.length === 0" class="cs-empty">
          No collections available.
        </div>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.collection-selector {
  display: inline-flex;
  position: relative;
}

/* ─── Tag (selected state) ─── */
.cs-tag {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  padding: 2px 4px 2px 0;
  border-radius: var(--radius-md, 8px);
  transition: background var(--transition-fast, 150ms);
  user-select: none;
}

.cs-tag:hover {
  background: var(--color-muted, #1a1a32);
}

.cs-clear {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border: none;
  border-radius: var(--radius-full, 9999px);
  background: transparent;
  color: var(--color-muted-foreground, #7d7da3);
  cursor: pointer;
  padding: 0;
  line-height: 1;
  transition: all var(--transition-fast, 150ms);
  flex-shrink: 0;
}

.cs-clear:hover {
  background: color-mix(in srgb, var(--color-destructive, #ef4444) 15%, transparent);
  color: var(--color-destructive, #ef4444);
}

/* ─── Trigger (no selection) ─── */
.cs-trigger {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  height: 36px;
  padding: var(--space-1, 4px) var(--space-3, 12px);
  background: var(--color-card, #16162e);
  border: 1px solid var(--color-input, #3a3a5e);
  border-radius: var(--radius-md, 8px);
  color: var(--color-foreground, #e0e0f0);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  cursor: pointer;
  outline: none;
  transition: border-color var(--transition-fast, 150ms);
  width: 360px;
  justify-content: space-between;
  line-height: 1;
}

.cs-trigger:hover {
  border-color: var(--color-primary, #4a6fff);
}

.cs-placeholder {
  color: var(--color-muted-foreground, #7d7da3);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cs-chevron {
  color: var(--color-muted-foreground, #7d7da3);
  font-size: 10px;
  line-height: 1;
  flex-shrink: 0;
  transition: transform var(--transition-fast, 150ms);
}

.cs-tag:hover .cs-chevron,
.cs-trigger:hover .cs-chevron,
.cs-trigger:focus-within .cs-chevron {
  transform: rotate(180deg);
}

/* ─── Dropdown ─── */
.cs-dropdown {
  background: var(--color-popover, #1e1e3a);
  border: 1px solid var(--color-border, #2a2a4e);
  border-radius: var(--radius-md, 8px);
  box-shadow: var(--shadow-md, 0 4px 12px rgba(0, 0, 0, 0.2));
  overflow: hidden;
  padding: var(--space-1, 4px) 0;
  position: fixed;
  z-index: 1000;
  min-width: 200px;
}

.cs-option {
  display: flex;
  align-items: center;
  gap: var(--space-2, 8px);
  width: 100%;
  padding: var(--space-1, 4px) var(--space-3, 12px);
  background: transparent;
  border: none;
  color: var(--color-foreground, #e0e0f0);
  font-family: var(--font-family);
  font-size: var(--font-size-sm, 13px);
  cursor: pointer;
  text-align: left;
  outline: none;
  transition: background var(--transition-fast, 150ms);
  line-height: 1;
  min-height: 32px;
}

.cs-option:hover {
  background: var(--color-muted, #1a1a32);
  color: var(--color-primary, #4a6fff);
}

.cs-option--selected {
  color: var(--color-primary, #4a6fff);
}

.cs-option-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cs-option-count {
  font-size: var(--font-size-2xs, 11px);
  color: var(--color-muted-foreground, #7d7da3);
  opacity: 0.7;
  flex-shrink: 0;
}

.cs-check {
  color: var(--color-primary, #4a6fff);
  font-weight: 600;
  flex-shrink: 0;
}

.cs-empty {
  padding: var(--space-3, 12px);
  text-align: center;
  color: var(--color-muted-foreground, #7d7da3);
  font-size: var(--font-size-xs, 12px);
}

/* ─── Responsive ─── */
@media (max-width: 768px) {
  .cs-trigger {
    width: auto;
    min-width: 140px;
  }
}
</style>

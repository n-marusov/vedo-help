<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from 'vue';

const props = withDefaults(
  defineProps<{
    modelValue: string | null | undefined;
    options: Array<{ value: string; label: string }>;
    placeholder?: string;
  }>(),
  {
    placeholder: 'Select...',
  },
);

const emit = defineEmits<{
  'update:modelValue': [value: string | null];
}>();

const open = ref(false);
const triggerRef = ref<HTMLElement | null>(null);
const dropdownRef = ref<HTMLElement | null>(null);
const dropdownStyle = ref<Record<string, string>>({});

const isPlaceholder = computed(() => props.modelValue == null || props.modelValue === '');

const selectedLabel = computed(() => {
  if (isPlaceholder.value) return props.placeholder;
  const match = props.options.find((o) => o.value === props.modelValue);
  return match ? match.label : props.placeholder;
});

async function toggle() {
  open.value = !open.value;
  if (open.value) {
    await nextTick();
    updateDropdownPosition();
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

function handleViewportChange() {
  if (open.value) {
    updateDropdownPosition();
  }
}

function select(value: string) {
  // If the same option is clicked again, deselect (emit null)
  if (value === props.modelValue) {
    emit('update:modelValue', null);
  } else {
    emit('update:modelValue', value);
  }
  open.value = false;
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
  window.addEventListener('resize', handleViewportChange);
  window.addEventListener('scroll', handleViewportChange, true);
});

onUnmounted(() => {
  document.removeEventListener('click', handleClickOutside);
  document.removeEventListener('keydown', handleKeydown);
  window.removeEventListener('resize', handleViewportChange);
  window.removeEventListener('scroll', handleViewportChange, true);
});
watch(
  () => props.options,
  async () => {
    if (open.value) {
      await nextTick();
      updateDropdownPosition();
    }
  },
);
</script>

<template>
  <div class="v-select">
    <button
      ref="triggerRef"
      class="v-select__trigger"
      :class="{ 'v-select__trigger--open': open }"
      type="button"
      @click="toggle"
    >
      <span
        class="v-select__value"
        :class="{ 'v-select__placeholder': isPlaceholder }"
      >
        {{ selectedLabel }}
      </span>
      <span class="v-select__chevron">▾</span>
    </button>

    <Teleport to="body">
      <div
        v-if="open"
        ref="dropdownRef"
        class="v-select__dropdown"
        data-testid="collection-select-dropdown"
        :style="dropdownStyle"
      >
        <button
          v-for="opt in options"
          :key="opt.value"
          class="v-select__option"
          :class="{ 'v-select__option--selected': opt.value === modelValue }"
          type="button"
          @click="select(opt.value)"
        >
          <span class="v-select__option-label">{{ opt.label }}</span>
          <span v-if="opt.value === modelValue" class="v-select__check">✓</span>
        </button>
      </div>
    </Teleport>
  </div>
</template>

<style scoped>
.v-select {
  display: inline-block;
  position: relative;
}

/* ── Trigger ── */
.v-select__trigger {
  align-items: center;
  background: var(--color-card);
  border: 1px solid var(--color-input);
  border-radius: var(--radius-md);
  color: var(--color-foreground);
  cursor: pointer;
  display: flex;
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  gap: var(--space-2);
  height: 36px;
  justify-content: space-between;
  line-height: 1;
  min-width: 200px;
  outline: none;
  padding: var(--space-1) var(--space-3);
  transition: border-color var(--transition-fast);
  width: 100%;
}

.v-select__trigger:hover,
.v-select__trigger--open {
  border-color: var(--color-primary);
}

.v-select__value {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.v-select__placeholder {
  color: var(--color-muted-foreground);
}

.v-select__chevron {
  color: var(--color-muted-foreground);
  flex: 0 0 auto;
  font-size: 10px;
  line-height: 1;
  transition: transform var(--transition-fast);
}

.v-select__trigger--open .v-select__chevron {
  transform: rotate(180deg);
}

/* ── Dropdown ── */
.v-select__dropdown {
  background: var(--color-popover);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-md);
  margin-top: var(--space-1);
  overflow: hidden;
  padding: var(--space-1) 0;
  position: fixed;
  z-index: 1000;
}

.v-select__option {
  align-items: center;
  background: transparent;
  border: none;
  color: var(--color-foreground);
  cursor: pointer;
  display: flex;
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  gap: var(--space-2);
  justify-content: space-between;
  line-height: 1;
  outline: none;
  padding: var(--space-1) var(--space-3);
  text-align: left;
  transition:
    background var(--transition-fast),
    color var(--transition-fast);
  width: 100%;
}

.v-select__option:hover,
.v-select__option--selected {
  color: var(--color-primary);
}

.v-select__option:hover {
  background: var(--color-muted);
}

.v-select__check {
  color: var(--color-primary);
  flex: 0 0 auto;
  font-weight: 600;
}
</style>

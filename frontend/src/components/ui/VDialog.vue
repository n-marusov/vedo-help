<script setup lang="ts">
import VButton from "./VButton.vue";

withDefaults(
  defineProps<{
    open: boolean;
    title?: string;
    description?: string;
    confirmText?: string;
    cancelText?: string;
    variant?: "default" | "destructive";
  }>(),
  {
    title: "",
    description: "",
    confirmText: "Confirm",
    cancelText: "Cancel",
    variant: "default",
  },
);

const emit = defineEmits<{
  close: [];
  confirm: [];
}>();
</script>

<template>
  <Teleport to="body">
    <div
      v-if="open"
      class="dialog-overlay"
      data-testid="confirm-dialog"
      @click.self="emit('close')"
    >
      <div class="dialog-card" role="dialog" :aria-label="title">
        <div v-if="title" class="dialog-header">
          <h2 class="dialog-title">{{ title }}</h2>
        </div>

        <div v-if="description || $slots.default" class="dialog-body">
          <p v-if="description" class="dialog-description">{{ description }}</p>
          <slot />
        </div>

        <div class="dialog-actions">
          <slot name="actions">
            <VButton
              variant="outline"
              data-testid="btn-dialog-cancel"
              @click="emit('close')"
            >
              {{ cancelText }}
            </VButton>
            <VButton
              :variant="variant === 'destructive' ? 'destructive' : 'primary'"
              data-testid="btn-dialog-confirm"
              @click="emit('confirm')"
            >
              {{ confirmText }}
            </VButton>
          </slot>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.dialog-card {
  width: 420px;
  max-width: 90vw;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl, 16px);
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 18px;
  box-shadow: var(--shadow-xl, 0 20px 60px rgba(0, 0, 0, 0.4));
}

.dialog-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.dialog-title {
  margin: 0;
  font-size: var(--font-size-xl, 18px);
  font-weight: 700;
  color: var(--color-foreground);
  font-family: var(--font-family);
}

.dialog-body {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.dialog-description {
  margin: 0;
  font-size: var(--font-size-xs, 12px);
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  line-height: 1.5;
}

.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 12px;
}
</style>

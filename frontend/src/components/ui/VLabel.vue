<script setup lang="ts">
withDefaults(
  defineProps<{
    required?: boolean;
  }>(),
  {
    required: false,
  },
);
</script>

<template>
  <label class="v-label">
    <slot />
    <span v-if="required" class="v-label__asterisk" aria-hidden="true">*</span>
  </label>
</template>

<style scoped>
.v-label {
  color: var(--color-foreground);
  display: inline-block;
  font-family: var(--font-family);
  font-size: var(--font-size-xs);
  font-weight: 500;
  letter-spacing: 0.5px;
  line-height: 1.4;
}

.v-label__asterisk {
  color: var(--color-destructive);
  margin-left: 2px;
}
</style>

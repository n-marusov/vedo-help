<script setup lang="ts">
import UserAvatar from '@/components/ui/UserAvatar.vue';

console.debug('[FIX:avatar-e2e] rendering UserAvatar preview route');
</script>

<template>
  <section class="avatar-preview" aria-labelledby="avatar-preview-title">
    <h1 id="avatar-preview-title">UserAvatar preview</h1>
    <p class="avatar-preview__description">
      Task 1.2 preview for the standalone SVG-based chat avatar component.
    </p>

    <div class="avatar-preview__grid" data-testid="avatar-preview-grid">
      <div class="avatar-preview__item">
        <UserAvatar role="user" size="sm" data-testid="avatar-user-sm" />
        <span>User small</span>
      </div>
      <div class="avatar-preview__item">
        <UserAvatar role="user" data-testid="avatar-user-md" />
        <span>User medium</span>
      </div>
      <div class="avatar-preview__item">
        <UserAvatar role="assistant" size="lg" data-testid="avatar-assistant-lg" />
        <span>Assistant large</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.avatar-preview {
  min-height: 100%;
  padding: 2rem;
}

.avatar-preview__description {
  color: var(--msg-time-color);
  margin-bottom: 1.5rem;
}

.avatar-preview__grid {
  align-items: center;
  display: flex;
  flex-wrap: wrap;
  gap: 1rem;
}

.avatar-preview__item {
  align-items: center;
  background: var(--msg-assistant-bg);
  border-radius: var(--msg-radius-assistant);
  color: var(--msg-assistant-text);
  display: flex;
  gap: 0.75rem;
  padding: 0.75rem 1rem;
}
</style>

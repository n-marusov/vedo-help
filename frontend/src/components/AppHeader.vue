<script setup>
/**
 * Global page header synced with design/ui-kit.lib.pen → Component/PageHeader.
 * Provides persistent VEDO branding and a single theme toggle for app pages.
 */
import VThemeToggle from '@/components/ui/VThemeToggle.vue';
import { onMounted } from 'vue';

onMounted(() => {
  console.debug('[AppHeader] mounted: global header with theme toggle is visible');
});
</script>

<template>
  <header class="app-header" data-testid="app-header">
    <div class="app-header__brand" aria-label="VEDO home">
      <span class="app-header__logo" aria-hidden="true">💬</span>
      <span class="app-header__name">VEDO</span>
    </div>

    <div class="app-header__actions" data-testid="app-header-actions">
      <VThemeToggle />
      <span class="app-header__user" aria-label="User menu" role="img">👤</span>
    </div>
  </header>
</template>

<style scoped>
.app-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 60px;
  min-height: 60px;
  padding: 0 24px;
  background: var(--color-card);
  border-bottom: 1px solid var(--color-border);
  color: var(--color-foreground);
  flex-shrink: 0;
}

.app-header__brand {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  min-width: 0;
}

.app-header__logo {
  color: var(--color-primary);
  font-family: var(--font-family);
  font-size: 18px;
  font-weight: 700;
  line-height: 1;
}

.app-header__name {
  color: var(--color-foreground);
  font-family: var(--font-family);
  font-size: 16px;
  font-weight: 700;
  letter-spacing: 0.01em;
}

.app-header__actions {
  display: inline-flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
}

.app-header__user {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm);
  background: var(--color-card);
  color: var(--color-muted-foreground);
  font-size: 14px;
  line-height: 1;
}

@media (max-width: 480px) {
  .app-header {
    padding: 0 var(--space-4);
  }
}
</style>

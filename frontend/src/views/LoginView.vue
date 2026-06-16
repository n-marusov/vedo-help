<script setup>
// Plain JS per project convention: template-only or composition-api views
// use `<script setup>` without `lang="ts"`. Sub-components in `components/`
// may opt into TypeScript with `lang="ts"` when they validate complex props.
import LoginButtons from '@/components/LoginButtons.vue';
import VThemeToggle from '@/components/ui/VThemeToggle.vue';
import { onMounted } from 'vue';

onMounted(() => {
  console.debug('[LoginView] mounted: login page with OAuth providers');
});
</script>

<template>
  <div class="login-view" data-testid="login-page">
    <div class="login-card" data-testid="login-card">
      <!-- Header row -->
      <div class="header-row" data-testid="login-header-row">
        <div class="logo-box">
          <span class="logo-text">V</span>
        </div>
        <div class="text-col">
          <h1 class="title" data-testid="login-title">
            Virtual Environment for Developing Ontologies
          </h1>
          <p class="subtitle">Build, connect, and share knowledge at scale</p>
        </div>
        <VThemeToggle />
      </div>

      <!-- OAuth provider buttons -->
      <LoginButtons />
    </div>
  </div>
</template>

<style scoped>
.login-view {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  min-height: 100dvh;
  background: var(--color-background);
  padding: var(--space-4);
}

.login-card {
  width: 100%;
  max-width: 720px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: var(--space-8);
  display: flex;
  flex-direction: column;
  gap: var(--space-6);
}

/* ── Header row ── */

.header-row {
  display: flex;
  align-items: center;
  gap: var(--space-4);
}

.logo-box {
  flex-shrink: 0;
  width: 56px;
  height: 56px;
  background: var(--color-primary);
  border-radius: var(--radius-lg);
  display: flex;
  align-items: center;
  justify-content: center;
}

.logo-text {
  font-family: var(--font-family);
  font-size: 28px;
  font-weight: bold;
  color: var(--color-primary-foreground);
  line-height: 1;
}

.text-col {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.title {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-xl);
  color: var(--color-foreground);
  font-weight: normal;
  line-height: 1.3;
}

.subtitle {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  color: var(--color-muted-foreground);
  line-height: 1.4;
}
</style>

<script setup>
import { handleCallback } from '@/composables/useOidcAuth';
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';

const router = useRouter();
const error = ref('');

onMounted(async () => {
  console.debug('[CallbackView] mounted: processing OAuth callback');
  try {
    await handleCallback();
    console.debug('[CallbackView] OAuth callback processed successfully, redirecting to chat');
    router.replace('/');
  } catch (err) {
    const message = err instanceof Error ? err.message : 'Authentication failed';
    console.error('[CallbackView] OAuth callback error:', message);
    error.value = message;
  }
});
</script>

<template>
  <div class="callback-view" data-testid="callback-page">
    <div class="callback-card" data-testid="callback-card">
      <div v-if="!error" class="callback-spinner">
        <div class="spinner" aria-label="Signing in..." />
        <p class="callback-text">Signing in&hellip;</p>
      </div>
      <div v-else class="callback-error">
        <p class="error-title">Authentication Failed</p>
        <p class="error-message">{{ error }}</p>
        <router-link to="/login" class="error-link">Back to Login</router-link>
      </div>
    </div>
  </div>
</template>

<style scoped>
.callback-view {
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: 100vh;
  min-height: 100dvh;
  background: var(--color-background);
  padding: var(--space-4);
}

.callback-card {
  width: 100%;
  max-width: 400px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-xl);
  padding: var(--space-8);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-4);
}

.callback-spinner {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-4);
}

.spinner {
  width: 32px;
  height: 32px;
  border: 3px solid var(--color-border);
  border-top-color: var(--color-primary);
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.callback-text {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  color: var(--color-muted-foreground);
}

.callback-error {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  text-align: center;
}

.error-title {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-lg);
  color: var(--color-destructive);
  font-weight: 600;
}

.error-message {
  margin: 0;
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  color: var(--color-muted-foreground);
  word-break: break-word;
}

.error-link {
  margin-top: var(--space-2);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  color: var(--color-primary);
  text-decoration: none;
}

.error-link:hover {
  text-decoration: underline;
}
</style>

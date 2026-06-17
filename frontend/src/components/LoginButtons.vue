<script setup lang="ts">
import { redirectToKeycloak } from '@/composables/useOidcAuth';
import { ref } from 'vue';

defineOptions({ name: 'LoginButtons' });

const isRedirecting = ref(false);

const providers = [
  { id: 'vk', label: 'Continue with VK ID' },
  { id: 'yandex', label: 'Continue with Yandex ID' },
  { id: 'mailru', label: 'Continue with Mail.ru' },
  { id: 'google', label: 'Continue with Google' },
  { id: 'corp-sso', label: 'Corporate SSO (SAML/OIDC)' },
];

async function handleOAuth() {
  if (isRedirecting.value) return;
  isRedirecting.value = true;

  await redirectToKeycloak();
}
</script>

<template>
  <div class="oauth-group">
    <button
      v-for="p in providers"
      :key="p.id"
      class="oauth-btn"
      :class="{ 'oauth-btn--loading': isRedirecting }"
      :disabled="isRedirecting"
      :data-testid="`btn-login-${p.id}`"
      @click="handleOAuth"
    >
      <!-- VK ID icon -->
      <svg
        v-if="p.id === 'vk'"
        class="provider-icon"
        viewBox="0 0 20 20"
        width="18"
        height="18"
        aria-hidden="true"
      >
        <rect width="20" height="20" rx="4" fill="#4A76A8" />
        <text
          x="10"
          y="13"
          text-anchor="middle"
          fill="#fff"
          font-size="10"
          font-weight="bold"
          font-family="'IBM Plex Mono', monospace"
        >
          VK
        </text>
      </svg>

      <!-- Yandex ID icon -->
      <svg
        v-else-if="p.id === 'yandex'"
        class="provider-icon"
        viewBox="0 0 20 20"
        width="18"
        height="18"
        aria-hidden="true"
      >
        <circle cx="10" cy="10" r="10" fill="#FC3F1D" />
        <text
          x="10"
          y="13"
          text-anchor="middle"
          fill="#fff"
          font-size="10"
          font-weight="bold"
          font-family="'IBM Plex Mono', monospace"
        >
          Ya
        </text>
      </svg>

      <!-- Mail.ru icon -->
      <svg
        v-else-if="p.id === 'mailru'"
        class="provider-icon"
        viewBox="0 0 20 20"
        width="18"
        height="18"
        aria-hidden="true"
      >
        <rect width="20" height="20" rx="4" fill="#005FF9" />
        <rect
          x="3"
          y="6"
          width="14"
          height="9"
          rx="1.5"
          fill="none"
          stroke="#fff"
          stroke-width="1.5"
        />
        <path d="M3 7l7 5 7-5" fill="none" stroke="#fff" stroke-width="1.5" />
      </svg>

      <!-- Google icon -->
      <svg
        v-else-if="p.id === 'google'"
        class="provider-icon"
        viewBox="0 0 20 20"
        width="18"
        height="18"
        aria-hidden="true"
      >
        <circle cx="10" cy="10" r="10" fill="#fff" />
        <path
          d="M10 8.182v3.818h3.182a3.636 3.636 0 0 1-1.576 2.388l2.545 1.977A6.364 6.364 0 0 0 16.364 10c0-.455-.045-.909-.091-1.364H10z"
          fill="#4285F4"
        />
        <path
          d="M5.909 11.891l-1.818 1.336A6.404 6.404 0 0 0 10 16.364c1.818 0 3.455-.636 4.727-1.636l-2.545-1.977c-.727.455-1.636.727-2.727.727-1.909 0-3.545-1.273-4.136-3.045l-1.818 1.336z"
          fill="#34A853"
        />
        <path
          d="M5.909 8.637C6.5 6.864 8.136 5.636 10 5.636c1.045 0 1.955.364 2.636 1.045l2-1.955A6.366 6.366 0 0 0 10 3.636c-2.545 0-4.727 1.409-5.909 3.5l1.818 1.5z"
          fill="#EA4335"
        />
      </svg>

      <!-- Corporate SSO icon -->
      <svg
        v-else
        class="provider-icon"
        viewBox="0 0 20 20"
        width="18"
        height="18"
        aria-hidden="true"
      >
        <path
          d="M10 1.25l7.5 3.75v5c0 4.145-3.355 7.5-7.5 7.5s-7.5-3.355-7.5-7.5V5L10 1.25z"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
        />
        <path
          d="M8 10.5l1.5 1.5 3-3.5"
          fill="none"
          stroke="currentColor"
          stroke-width="1.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        />
      </svg>

      <span>{{ p.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.oauth-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}

.oauth-btn {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  width: 100%;
  height: 44px;
  padding: 0 var(--space-4);
  background: transparent;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  color: var(--color-foreground);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  cursor: pointer;
  text-align: left;
  transition:
    background var(--transition-fast),
    border-color var(--transition-fast);
}

.oauth-btn:hover {
  background: var(--color-muted);
}

.oauth-btn:active {
  background: var(--color-secondary);
}

.provider-icon {
  flex-shrink: 0;
  display: block;
}
</style>

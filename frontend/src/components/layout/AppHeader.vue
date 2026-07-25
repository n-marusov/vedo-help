<script setup lang="ts">
import { decodeToken } from '@/api/auth';
/**
 * Global page header synced with design/ui-kit.lib.pen → Component/PageHeader.
 * Provides persistent VEDO branding, a theme toggle, and user menu for app pages.
 */
import { getAccessToken } from '@/api/client';
import VThemeToggle from '@/components/ui/VThemeToggle.vue';
import { logout } from '@/composables/useOidcAuth';
import { userName } from '@/stores/auth';
import { computed, nextTick, onMounted, onUnmounted, ref } from 'vue';
import { useRouter } from 'vue-router';

const userMenuOpen = ref(false);
const userButtonRef = ref<HTMLElement | null>(null);
const userDropdownRef = ref<HTMLElement | null>(null);
const userDropdownStyle = ref<Record<string, string>>({});

const userInfo = computed(() => {
  const token = getAccessToken();
  if (!token) return null;
  const decoded = decodeToken(token);
  const roles = (decoded?.realm_access as { roles?: string[] })?.roles ?? [];
  return {
    name: userName.value || 'User',
    isAdmin: roles.includes('admin') || roles.includes('vedo-admin'),
  };
});

onMounted(() => {
  document.addEventListener('click', closeMenuOnOutside);
  window.addEventListener('resize', handleViewportChange);
  window.addEventListener('scroll', handleViewportChange, true);
});

async function toggleUserMenu() {
  userMenuOpen.value = !userMenuOpen.value;
  if (userMenuOpen.value) {
    await nextTick();
    updateUserDropdownPosition();
  }
}

function updateUserDropdownPosition() {
  if (!userButtonRef.value) return;

  const rect = userButtonRef.value.getBoundingClientRect();
  userDropdownStyle.value = {
    minWidth: '180px',
    position: 'fixed',
    right: `${Math.max(window.innerWidth - rect.right, 8)}px`,
    top: `${rect.bottom + 6}px`,
  };
}

function handleViewportChange() {
  if (userMenuOpen.value) {
    updateUserDropdownPosition();
  }
}

function handleLogout() {
  userMenuOpen.value = false;
  logout();
}

const router = useRouter();
function navigateToAdmin() {
  userMenuOpen.value = false;
  router.push('/admin');
}

function closeMenuOnOutside(e: MouseEvent) {
  if (userMenuOpen.value) {
    const target = e.target as Node;
    if (!userButtonRef.value?.contains(target) && !userDropdownRef.value?.contains(target)) {
      userMenuOpen.value = false;
    }
  }
}

onUnmounted(() => {
  document.removeEventListener('click', closeMenuOnOutside);
  window.removeEventListener('resize', handleViewportChange);
  window.removeEventListener('scroll', handleViewportChange, true);
});
</script>

<template>
  <header class="app-header" data-testid="app-header">
    <router-link to="/" class="app-header__brand" aria-label="VEDO home">
      <span class="app-header__logo" aria-hidden="true">
        <svg
          class="app-header__icon app-header__icon--brand"
          fill="none"
          viewBox="0 0 24 24"
          xmlns="http://www.w3.org/2000/svg"
        >
          <path
            d="M21 11.5a8.4 8.4 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.4 8.4 0 0 1-3.8-.9L3 21l1.9-5.7a8.4 8.4 0 0 1-.9-3.8 8.5 8.5 0 0 1 17 0Z"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.8"
          />
          <path
            d="M8.5 12h.01M12 12h.01M15.5 12h.01"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2.2"
          />
        </svg>
      </span>
      <span class="app-header__name">VEDO</span>
    </router-link>

    <div class="app-header__actions" data-testid="app-header-actions">
      <VThemeToggle />
      <!-- User menu dropdown -->
      <div
        v-if="userInfo"
        class="app-header__user-menu"
        data-testid="user-menu"
      >
        <button
          ref="userButtonRef"
          class="app-header__user-btn"
          aria-label="Open user menu"
          @click="toggleUserMenu"
        >
          <span class="app-header__user-avatar" aria-hidden="true">
            <svg
              class="app-header__icon"
              fill="none"
              viewBox="0 0 24 24"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                d="M20 21a8 8 0 0 0-16 0"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1.8"
              />
              <circle
                cx="12"
                cy="8"
                r="4"
                stroke="currentColor"
                stroke-linecap="round"
                stroke-linejoin="round"
                stroke-width="1.8"
              />
            </svg>
          </span>
        </button>
        <Teleport to="body">
          <div
            v-if="userMenuOpen"
            ref="userDropdownRef"
            class="app-header__dropdown"
            data-testid="user-dropdown"
            :style="userDropdownStyle"
          >
            <div class="app-header__dropdown-header">
              <span class="app-header__dropdown-name">{{ userInfo.name }}</span>
            </div>
            <div class="app-header__dropdown-divider" />
            <button
              v-if="userInfo.isAdmin"
              class="app-header__dropdown-item"
              @click="navigateToAdmin"
            >
              Admin Panel
            </button>
            <button
              class="app-header__dropdown-item app-header__dropdown-item--danger"
              @click="handleLogout"
            >
              Sign Out
            </button>
          </div>
        </Teleport>
      </div>
      <span v-else class="app-header__user" aria-label="User menu" role="img">
        <svg
          class="app-header__icon"
          fill="none"
          viewBox="0 0 24 24"
          xmlns="http://www.w3.org/2000/svg"
          aria-hidden="true"
        >
          <path
            d="M20 21a8 8 0 0 0-16 0"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.8"
          />
          <circle
            cx="12"
            cy="8"
            r="4"
            stroke="currentColor"
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="1.8"
          />
        </svg>
      </span>
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
  text-decoration: none;
  color: inherit;
  cursor: pointer;
}

.app-header__logo {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 18px;
  height: 18px;
  color: var(--color-primary);
  line-height: 1;
}

.app-header__icon {
  display: block;
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.app-header__icon--brand {
  width: 18px;
  height: 18px;
}

.app-header__brand:hover {
  opacity: 0.8;
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

/*
.app-header__user — fallback for unauthenticated state.
*/

.app-header__user {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--color-muted-foreground);
  line-height: 1;
}

/* ── User Menu Button ── */

.app-header__user-menu {
  position: relative;
}

.app-header__user-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: var(--color-muted-foreground);
  cursor: pointer;
  line-height: 1;
  padding: 0;
  background: none;
  border: none;
  transition: color var(--transition-fast);
}

.app-header__user-btn:hover {
  color: var(--color-foreground);
}

.app-header__user-avatar {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  line-height: 1;
}

/* ── Dropdown ── */

.app-header__dropdown {
  position: fixed;
  min-width: 180px;
  background: var(--color-popover);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-md);
  box-shadow: var(--shadow-lg);
  z-index: 100;
  overflow: hidden;
}

.app-header__dropdown-header {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: var(--space-3) var(--space-4);
}

.app-header__dropdown-name {
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  color: var(--color-foreground);
  font-weight: 600;
}

.app-header__dropdown-provider {
  font-family: var(--font-family);
  font-size: var(--font-size-2xs);
  color: var(--color-muted-foreground);
  text-transform: capitalize;
}

.app-header__dropdown-divider {
  height: 1px;
  background: var(--color-border);
  margin: 0;
}

.app-header__dropdown-item {
  display: block;
  width: 100%;
  padding: var(--space-2) var(--space-4);
  font-family: var(--font-family);
  font-size: var(--font-size-sm);
  color: var(--color-foreground);
  background: none;
  border: none;
  text-align: left;
  cursor: pointer;
  transition: background var(--transition-fast);
}

.app-header__dropdown-item:hover {
  background: var(--color-muted);
}

.app-header__dropdown-item--danger {
  color: var(--color-destructive);
}

@media (max-width: 480px) {
  .app-header {
    padding: 0 var(--space-4);
  }
}
</style>

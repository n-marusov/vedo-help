<script setup>
import { getAccessToken } from '@/api/client';
/**
 * Global page header synced with design/ui-kit.lib.pen → Component/PageHeader.
 * Provides persistent VEDO branding, a theme toggle, and user menu for app pages.
 */
import VThemeToggle from '@/components/ui/VThemeToggle.vue';
import { logout } from '@/composables/useOidcAuth';
import { userName, userProvider } from '@/stores/auth';
import { computed, onMounted, onUnmounted, ref } from 'vue';

const userMenuOpen = ref(false);

const userInfo = computed(() => {
  const token = getAccessToken();
  if (!token) return null;
  return {
    name: userName.value || 'User',
    provider: userProvider.value,
  };
});

onMounted(() => {
  console.debug('[AppHeader] mounted: global header with theme toggle is visible');
  document.addEventListener('click', closeMenuOnOutside);
});

function toggleUserMenu() {
  userMenuOpen.value = !userMenuOpen.value;
}

function handleLogout() {
  userMenuOpen.value = false;
  logout();
}

function closeMenuOnOutside(e) {
  // Close the dropdown when clicking outside
  if (userMenuOpen.value) {
    const target = e.target;
    if (!target.closest('.app-header__user-menu') && !target.closest('.app-header__user')) {
      userMenuOpen.value = false;
    }
  }
}

onUnmounted(() => {
  document.removeEventListener('click', closeMenuOnOutside);
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
      <!-- User menu dropdown -->
      <div
        v-if="userInfo"
        class="app-header__user-menu"
        data-testid="user-menu"
      >
        <button
          class="app-header__user-btn"
          aria-label="Open user menu"
          @click="toggleUserMenu"
        >
          <span class="app-header__user-avatar">👤</span>
        </button>
        <div
          v-if="userMenuOpen"
          class="app-header__dropdown"
          data-testid="user-dropdown"
        >
          <div class="app-header__dropdown-header">
            <span class="app-header__dropdown-name">{{ userInfo.name }}</span>
            <span
              v-if="userInfo.provider"
              class="app-header__dropdown-provider"
              >{{ userInfo.provider }}</span
            >
          </div>
          <div class="app-header__dropdown-divider" />
          <button
            class="app-header__dropdown-item app-header__dropdown-item--danger"
            @click="handleLogout"
          >
            Sign Out
          </button>
        </div>
      </div>
      <span v-else class="app-header__user" aria-label="User menu" role="img"
        >👤</span
      >
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

/*
.app-header__user — fallback for unauthenticated state.
*/

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

/* ── User Menu Button ── */

.app-header__user-menu {
  position: relative;
}

.app-header__user-btn {
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
  cursor: pointer;
  line-height: 1;
  padding: 0;
  transition: background var(--transition-fast);
}

.app-header__user-btn:hover {
  background: var(--color-muted);
}

.app-header__user-avatar {
  line-height: 1;
}

/* ── Dropdown ── */

.app-header__dropdown {
  position: absolute;
  top: calc(100% + 6px);
  right: 0;
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

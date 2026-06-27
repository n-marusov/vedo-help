<script setup>
/**
 * VThemeToggle
 *
 * Design: ui-kit.lib.pen → Component/ToggleButton (A2iAv)
 * Toggles between dark and light themes.
 * Icon: outline sun in dark mode (switch to light), outline moon in light mode (switch to dark).
 */
import { useTheme } from '@/composables/useTheme';

const { isDark, toggleTheme } = useTheme();
</script>

<template>
  <button
    class="theme-toggle"
    data-testid="theme-toggle"
    :aria-label="isDark ? 'Switch to light theme' : 'Switch to dark theme'"
    :title="isDark ? 'Light theme' : 'Dark theme'"
    @click="toggleTheme"
  >
    <span class="theme-toggle__icon" aria-hidden="true">
      <svg
        v-if="isDark"
        fill="none"
        viewBox="0 0 24 24"
        xmlns="http://www.w3.org/2000/svg"
      >
        <circle
          cx="12"
          cy="12"
          r="4"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.8"
        />
        <path
          d="M12 2v2M12 20v2M4.93 4.93l1.41 1.41M17.66 17.66l1.41 1.41M2 12h2M20 12h2M4.93 19.07l1.41-1.41M17.66 6.34l1.41-1.41"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.8"
        />
      </svg>
      <svg
        v-else
        fill="none"
        viewBox="0 0 24 24"
        xmlns="http://www.w3.org/2000/svg"
      >
        <path
          d="M20.4 15.1A8.5 8.5 0 0 1 8.9 3.6 8.5 8.5 0 1 0 20.4 15.1Z"
          stroke="currentColor"
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="1.8"
        />
      </svg>
    </span>
  </button>
</template>

<style scoped>
.theme-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  color: var(--color-muted-foreground);
  line-height: 1;
  outline: none;
  user-select: none;
  transition: color var(--transition-fast, 150ms ease);
  padding: 0;
  background: none;
  border: none;
  border-radius: 0;
}

.theme-toggle:hover {
  color: var(--color-foreground);
}

.theme-toggle:focus-visible {
  color: var(--color-primary);
}

.theme-toggle__icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  line-height: 1;
}

.theme-toggle__icon svg {
  display: block;
  width: 16px;
  height: 16px;
}

/* Hide on print */
@media print {
  .theme-toggle {
    display: none;
  }
}
</style>

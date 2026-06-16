<script setup>
/**
 * VThemeToggle
 *
 * Design: ui-kit.lib.pen → Component/ToggleButton (A2iAv)
 * Toggles between dark and light themes.
 * Icon: ☀️ in dark (switch to light), 🌙 in light (switch to dark).
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
      {{ isDark ? '☀️' : '🌙' }}
    </span>
  </button>
</template>

<style scoped>
.theme-toggle {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 4px;
  padding: 4px 8px;
  background: var(--color-card);
  border: 1px solid var(--color-border);
  border-radius: var(--radius-sm, 6px);
  cursor: pointer;
  color: var(--color-muted-foreground);
  font-family: var(--font-family);
  line-height: 1;
  transition:
    background var(--transition-fast, 150ms ease),
    border-color var(--transition-fast, 150ms ease);
  outline: none;
  user-select: none;
}

.theme-toggle:hover {
  background: var(--color-muted);
  border-color: var(--color-input);
}

.theme-toggle:focus-visible {
  box-shadow: 0 0 0 2px var(--color-primary);
}

.theme-toggle:active {
  background: var(--color-secondary);
}

.theme-toggle__icon {
  display: inline-block;
  font-size: 14px;
  line-height: 1;
}

/* Hide on print */
@media print {
  .theme-toggle {
    display: none;
  }
}
</style>

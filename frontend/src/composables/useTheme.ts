import { computed, ref, watch } from 'vue';

const STORAGE_KEY = 'vedo_theme';
const DARK = 'dark';
const LIGHT = 'light';

/**
 * Reactive theme composable.
 *
 * - Reads/writes localStorage under `vedo_theme`.
 * - Applies/removes `data-theme="light"` on `<html>`.
 * - Defaults to dark theme when no (or invalid) preference is stored.
 * - Shared across all callers via module-level singleton state.
 */

// ── Module-level singleton state ──
const currentTheme = ref<typeof DARK | typeof LIGHT>(DARK);

function loadTheme(): typeof DARK | typeof LIGHT {
  try {
    const stored = localStorage.getItem(STORAGE_KEY);
    if (stored === LIGHT || stored === DARK) return stored;
  } catch {
    // localStorage not available (SSR, private browsing restrictions)
  }
  return DARK;
}

function applyTheme(theme: typeof DARK | typeof LIGHT): void {
  const html = document.documentElement;
  if (theme === LIGHT) {
    html.setAttribute('data-theme', LIGHT);
  } else {
    html.removeAttribute('data-theme');
  }
}

function persistTheme(theme: typeof DARK | typeof LIGHT): void {
  try {
    localStorage.setItem(STORAGE_KEY, theme);
  } catch {
    // Silently ignore storage errors
  }
}

// ── Initialise on first import ──
const initial = loadTheme();
currentTheme.value = initial;
applyTheme(initial);

export function useTheme() {
  const isDark = computed(() => currentTheme.value === DARK);
  const isLight = computed(() => currentTheme.value === LIGHT);

  function toggleTheme(): void {
    const next = currentTheme.value === DARK ? LIGHT : DARK;
    currentTheme.value = next;
    applyTheme(next);
    persistTheme(next);
  }

  function setTheme(theme: typeof DARK | typeof LIGHT): void {
    if (theme !== DARK && theme !== LIGHT) return;
    currentTheme.value = theme;
    applyTheme(theme);
    persistTheme(theme);
  }

  // Sync to DOM on external changes (e.g. from another tab)
  watch(currentTheme, (val) => {
    applyTheme(val);
    persistTheme(val);
  });

  return {
    /** Reactive — current theme mode string */
    currentTheme,
    /** Reactive — `true` when dark theme is active */
    isDark,
    /** Reactive — `true` when light theme is active */
    isLight,
    /** Toggle between dark ↔ light */
    toggleTheme,
    /** Explicitly set a theme */
    setTheme,
  };
}

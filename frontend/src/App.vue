<script setup>
import AppHeader from '@/components/AppHeader.vue';
import { computed, onMounted } from 'vue';
import { useRoute } from 'vue-router';

const route = useRoute();
const showAppHeader = computed(() => route.name !== 'login');

onMounted(() => {
  console.debug('[App] mounted: root layout initialized with persistent header');
});
</script>

<template>
  <div class="app-layout">
    <AppHeader v-if="showAppHeader" />
    <main class="main-content">
      <router-view />
    </main>
  </div>
</template>

<style>
*,
*::before,
*::after {
  box-sizing: border-box;
}

html,
body {
  height: 100%;
  margin: 0;
  padding: 0;
  overflow: hidden;
}

body {
  font-family: var(--font-family);
  background-color: var(--color-background);
  color: var(--color-foreground);
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

#app {
  height: 100%;
}
</style>

<style scoped>
.app-layout {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow: hidden;
  background-color: var(--color-background);
}

.main-content {
  flex: 1;
  min-height: 0;
  overflow: hidden;
}
</style>

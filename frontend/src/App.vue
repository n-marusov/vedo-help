<script setup></script>

<template>
  <div class="app-layout">
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
  height: 100%;
  overflow: hidden;
  background-color: var(--color-background);
}

.main-content {
  flex: 1;
  overflow: hidden;
}
</style>

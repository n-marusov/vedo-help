<script setup lang="ts">
import { useRoute } from 'vue-router';

const route = useRoute();
</script>

<template>
  <div class="app-layout">
    <aside class="sidebar">
      <div class="sidebar-header">
        <h1 class="sidebar-title">VEDO hub</h1>
      </div>
      <nav class="sidebar-nav">
        <router-link to="/" class="nav-link" :class="{ active: route.name === 'chat' }">
          <span class="nav-icon">💬</span>
          Chat
        </router-link>
        <router-link to="/admin" class="nav-link" :class="{ active: route.name === 'admin' }">
          <span class="nav-icon">⚙️</span>
          Admin
        </router-link>
      </nav>
      <div class="sidebar-footer">
        <span class="version">v0.1.0</span>
      </div>
    </aside>
    <main class="main-content">
      <router-view />
    </main>
  </div>
</template>

<style scoped>
.app-layout {
  display: flex;
  height: 100vh;
  overflow: hidden;
}

.sidebar {
  width: 260px;
  min-width: 260px;
  background-color: #1a1a2e;
  color: #e0e0e0;
  display: flex;
  flex-direction: column;
  border-right: 1px solid #2a2a4e;
}

.sidebar-header {
  padding: 1.25rem;
  border-bottom: 1px solid #2a2a4e;
}

.sidebar-title {
  margin: 0;
  font-size: 1.25rem;
  font-weight: 700;
  color: #ffffff;
}

.sidebar-nav {
  flex: 1;
  padding: 0.75rem 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 0.15rem;
}

.nav-link {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.6rem 1.25rem;
  color: #8b8bbf;
  text-decoration: none;
  font-size: 0.9rem;
  font-weight: 500;
  border-right: 3px solid transparent;
  transition: all 0.2s;
}

.nav-link:hover {
  background: #222244;
  color: #c0c0e0;
}

.nav-link.active {
  background: #1a2a4e;
  color: #ffffff;
  border-right-color: #4a6fff;
}

.nav-icon {
  font-size: 1rem;
  width: 22px;
  text-align: center;
}

.sidebar-footer {
  padding: 0.75rem 1.25rem;
  border-top: 1px solid #2a2a4e;
}

.version {
  font-size: 0.7rem;
  color: #4a4a6a;
}

.main-content {
  flex: 1;
  overflow: hidden;
  background-color: #0f0f23;
  color: #e0e0e0;
}
</style>

import { URL, fileURLToPath } from 'node:url';
import vue from '@vitejs/plugin-vue';
import { defineConfig } from 'vite';

const keycloakProxyTarget = process.env.VITE_KEYCLOAK_PROXY_TARGET ?? 'http://localhost:8080';
const apiProxyTarget = process.env.VITE_API_PROXY_TARGET ?? 'http://localhost:3000';

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: {
      '@': fileURLToPath(new URL('./src', import.meta.url)),
    },
  },
  server: {
    proxy: {
      '/auth': {
        target: keycloakProxyTarget,
        changeOrigin: true,
        rewrite: (path) => path.replace(/^\/auth/, ''),
      },
      '/api': {
        target: apiProxyTarget,
        changeOrigin: true,
      },
    },
  },
  // Tree-shake console.debug and console.error calls in production builds.
  // Dev-mode logging stays intact for debugging.
  esbuild: {
    pure: ['console.debug', 'console.error'],
  },
});

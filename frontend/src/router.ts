import AdminView from '@/views/AdminView.vue';
import ChatView from '@/views/ChatView.vue';
import { createRouter, createWebHistory } from 'vue-router';

const routes = [
  {
    path: '/',
    name: 'chat',
    component: ChatView,
  },
  {
    path: '/admin',
    name: 'admin',
    component: AdminView,
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;

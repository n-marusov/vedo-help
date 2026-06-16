import AdminView from '@/views/AdminView.vue';
import AvatarPreviewView from '@/views/AvatarPreviewView.vue';
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
  {
    path: '/ui-preview/avatar',
    name: 'avatar-preview',
    component: AvatarPreviewView,
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

export default router;

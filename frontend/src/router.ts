import { restoreSession } from '@/composables/useOidcAuth';
import AdminView from '@/views/AdminView.vue';
import AvatarPreviewView from '@/views/AvatarPreviewView.vue';
import CallbackView from '@/views/CallbackView.vue';
import ChatView from '@/views/ChatView.vue';
import LoginView from '@/views/LoginView.vue';
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
  {
    path: '/login',
    name: 'login',
    component: LoginView,
  },
  {
    path: '/callback',
    name: 'callback',
    component: CallbackView,
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

// Attempt to restore a previously stored session on first navigation.
restoreSession();

/**
 * Navigation guard — redirect unauthenticated users to the login page
 * for protected routes, except the login and callback pages themselves.
 */
router.beforeEach((to, _from, next) => {
  const publicRoutes = ['login', 'callback', 'avatar-preview'];
  if (!publicRoutes.includes(to.name as string)) {
    const key = localStorage.getItem('vedo_auth_token');
    if (!key) {
      console.debug('[Router] No auth token found, redirecting to login');
      next({ name: 'login' });
      return;
    }
  }
  next();
});

export default router;

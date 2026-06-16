import { createPinia } from 'pinia';
import { createApp } from 'vue';
import './assets/chat-tokens.css';
import App from './App.vue';
import { logChatTokenValues } from './chatTokens';
import router from './router';

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount('#app');
logChatTokenValues();

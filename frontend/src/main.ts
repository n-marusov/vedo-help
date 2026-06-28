import { createPinia } from 'pinia';
import { createApp } from 'vue';
import './assets/design-tokens.css';
import './assets/chat-tokens.css';
import './telemetry';
import App from './App.vue';
import router from './router';

import { initTelemetry } from './telemetry';
initTelemetry();

const app = createApp(App);

app.use(createPinia());
app.use(router);

app.mount('#app');

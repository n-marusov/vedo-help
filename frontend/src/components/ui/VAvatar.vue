<script setup lang="ts">
import { computed } from 'vue';

type AvatarRole = 'user' | 'assistant';
type AvatarSize = 'sm' | 'md' | 'lg';

const props = withDefaults(
  defineProps<{
    role: AvatarRole;
    size?: AvatarSize;
  }>(),
  {
    size: 'md',
  },
);

const ariaLabel = computed(() =>
  props.role === 'assistant' ? 'VEDO assistant avatar' : 'User avatar',
);
</script>

<template>
  <span
    :aria-label="ariaLabel"
    class="v-avatar"
    :class="[`v-avatar--${role}`, `v-avatar--${size}`]"
    role="img"
  >
    <!-- User: person outline -->
    <svg
      v-if="role === 'user'"
      aria-hidden="true"
      class="v-avatar__icon"
      fill="none"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M12 12.25a4 4 0 1 0 0-8 4 4 0 0 0 0 8Z"
        fill="currentColor"
        opacity="0.96"
      />
      <path
        d="M4.75 20.25a7.25 7.25 0 0 1 14.5 0 1 1 0 0 1-1 1H5.75a1 1 0 0 1-1-1Z"
        fill="currentColor"
        opacity="0.86"
      />
    </svg>

    <!-- Assistant: bold V letter -->
    <span v-else aria-hidden="true" class="v-avatar__letter">V</span>
  </span>
</template>

<style scoped>
.v-avatar {
  align-items: center;
  border: 1px solid var(--color-border);
  border-radius: var(--radius-full);
  display: inline-flex;
  flex: 0 0 auto;
  justify-content: center;
  overflow: hidden;
}

/* ── Sizes ── */
.v-avatar--sm {
  height: 24px;
  width: 24px;
}

.v-avatar--md {
  height: 32px;
  width: 32px;
}

.v-avatar--lg {
  height: 40px;
  width: 40px;
}

/* ── Roles ── */
.v-avatar--user {
  background: var(--color-muted);
  color: #ffffff;
}

.v-avatar--assistant {
  background: var(--color-muted);
  color: #ffffff;
}

/* ── Icon ── */
.v-avatar__icon {
  display: block;
  height: 62.5%;
  width: 62.5%;
}

.v-avatar__letter {
  font-family: var(--font-family);
  font-weight: 700;
  line-height: 1;
}

.v-avatar--sm .v-avatar__letter {
  font-size: 13px;
}

.v-avatar--md .v-avatar__letter {
  font-size: 17px;
}

.v-avatar--lg .v-avatar__letter {
  font-size: 21px;
}
</style>

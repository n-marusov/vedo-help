<script setup lang="ts">
import { computed, onMounted } from 'vue';

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

const sizeMap: Record<AvatarSize, string> = {
  sm: 'calc(var(--avatar-size) * 0.75)',
  md: 'var(--avatar-size)',
  lg: 'calc(var(--avatar-size) * 1.25)',
};

const avatarClasses = computed(() => [
  'user-avatar',
  `user-avatar--${props.role}`,
  `user-avatar--${props.size}`,
]);

const avatarStyle = computed(() => ({
  '--user-avatar-size': sizeMap[props.size],
}));

const ariaLabel = computed(() =>
  props.role === 'assistant' ? 'VEDO assistant avatar' : 'User avatar',
);

onMounted(() => {});
</script>

<template>
  <span
    :aria-label="ariaLabel"
    :class="avatarClasses"
    :style="avatarStyle"
    role="img"
  >
    <svg
      v-if="role === 'user'"
      aria-hidden="true"
      class="user-avatar__icon"
      data-testid="user-avatar-icon"
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

    <svg
      v-else
      aria-hidden="true"
      class="user-avatar__icon user-avatar__icon--assistant"
      data-testid="assistant-avatar-icon"
      viewBox="0 0 24 24"
      xmlns="http://www.w3.org/2000/svg"
    >
      <title>VEDO</title>
      <text
        dominant-baseline="central"
        fill="currentColor"
        font-family="Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif"
        font-size="14"
        font-weight="800"
        text-anchor="middle"
        x="12"
        y="12.5"
      >
        V
      </text>
    </svg>
  </span>
</template>

<style scoped>
.user-avatar {
  align-items: center;
  border-radius: var(--avatar-radius);
  color: #ffffff;
  display: inline-flex;
  flex: 0 0 auto;
  height: var(--user-avatar-size);
  justify-content: center;
  overflow: hidden;
  width: var(--user-avatar-size);
}

.user-avatar--user {
  background: var(--avatar-user-bg);
}

.user-avatar--assistant {
  background: var(--avatar-assistant-bg);
}

.user-avatar__icon {
  display: block;
  height: 62.5%;
  width: 62.5%;
}

.user-avatar__icon--assistant {
  height: 72%;
  width: 72%;
}
</style>

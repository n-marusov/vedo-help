const chatTokenNames = [
  '--msg-gap',
  '--msg-padding-y',
  '--msg-padding-x',
  '--msg-radius-user',
  '--msg-radius-assistant',
  '--avatar-radius',
  '--anim-msg-enter-duration',
  '--anim-msg-enter-ease',
  '--anim-stream-duration',
  '--msg-user-bg',
  '--msg-assistant-bg',
  '--msg-user-text',
  '--msg-assistant-text',
  '--msg-time-color',
  '--avatar-user-bg',
  '--avatar-assistant-bg',
  '--avatar-size',
  '--max-msg-width',
  '--input-min-height',
] as const;

export function logChatTokenValues(): void {
  if (!import.meta.env.DEV || typeof window === 'undefined') return;

  const computedStyle = window.getComputedStyle(document.documentElement);
  const tokens = Object.fromEntries(
    chatTokenNames.map((name) => [name, computedStyle.getPropertyValue(name).trim()]),
  );

  console.debug('[chat-ui] resolved design tokens', tokens);
}

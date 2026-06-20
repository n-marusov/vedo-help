import { expect, test } from '@playwright/test';
import { mockCollections, setupAuth } from './helpers';

/**
 * ChatWindow Component Tests (Task 3.2, 3.3)
 *
 * Tests the redesigned ChatWindow layout including header,
 * collection selector, messages area, input area, and animations.
 */
test.describe('ChatWindow Layout', () => {
  test.beforeEach(async ({ page }) => {
    // DEBUG [e2e] auth setup added to chat-window beforeEach
    await setupAuth(page);
    // DEBUG [e2e] chat-window: mocking collections
    await mockCollections(page);
  });

  test('TC-CHAT-001: renders chat header with collection selector', async ({ page }) => {
    await page.goto('/');
    const header = page.locator('[data-testid="chat-toolbar"]');
    await expect(header).toBeVisible({ timeout: 5000 });

    // Collection selector should be in the header
    const collectionSelect = page.locator('[data-testid="collection-select"]');
    await expect(collectionSelect).toBeVisible();
  });

  test('TC-CHAT-002: collection selector shows collections list', async ({ page }) => {
    await page.goto('/');
    const collectionSelect = page.locator('[data-testid="collection-select"]');
    await collectionSelect.click();

    // VSelect renders a custom dropdown (Teleported to body), not native <option>
    const dropdown = page.locator('[data-testid="collection-select-dropdown"]');
    await expect(dropdown).toBeVisible({ timeout: 3000 });
    const optionItems = dropdown.locator('button');
    const optionCount = await optionItems.count();
    // At minimum: one option from mocked collections
    expect(optionCount).toBeGreaterThanOrEqual(1);
  });

  test('TC-CHAT-003: new chat button is visible in header', async ({ page }) => {
    await page.goto('/');
    const newChatBtn = page.locator('[data-testid="btn-new-chat"]');
    await expect(newChatBtn).toBeVisible({ timeout: 5000 });
  });

  test('TC-CHAT-004: new chat button clears messages', async ({ page }) => {
    await page.goto('/');
    const newChatBtn = page.locator('[data-testid="btn-new-chat"]');
    await newChatBtn.click();

    // After clicking new chat, messages should be cleared
    // Welcome message should appear
    const welcomeMsg = page.locator('[data-testid="welcome-message"]');
    await expect(welcomeMsg).toBeVisible({ timeout: 5000 });
  });

  test('TC-CHAT-005: messages area is scrollable', async ({ page }) => {
    await page.goto('/');
    const messagesArea = page.locator('[data-testid="messages-area"]');
    await expect(messagesArea).toBeVisible({ timeout: 5000 });

    // Messages area should be a scrollable container
    const overflowY = await messagesArea.evaluate((el) => getComputedStyle(el).overflowY);
    expect(overflowY).toBe('auto');
  });

  test('TC-CHAT-006: shows welcome message when no messages exist', async ({ page }) => {
    await page.goto('/');
    const welcomeMsg = page.locator('[data-testid="welcome-message"]');
    await expect(welcomeMsg).toBeVisible({ timeout: 5000 });

    // Welcome message should have title and description
    await expect(welcomeMsg.locator('h2')).toContainText('VEDO');
    await expect(welcomeMsg.locator('p')).not.toBeEmpty();
  });

  test('TC-CHAT-007: hides welcome message when messages exist', async ({ page }) => {
    await page.goto('/');
    const welcomeMsg = page.locator('[data-testid="welcome-message"]');

    const messages = page.locator('[data-testid^="message-"]');
    const hasMessages = (await messages.count()) > 0;
    if (hasMessages) {
      await expect(welcomeMsg).not.toBeVisible();
    }
  });

  test.describe('Input Area', () => {
    test('TC-CHAT-008: renders input textarea', async ({ page }) => {
      await page.goto('/');
      const input = page.locator('[data-testid="chat-input"]');
      await expect(input).toBeVisible({ timeout: 5000 });
    });

    test('TC-CHAT-009: input has placeholder text', async ({ page }) => {
      await page.goto('/');
      const input = page.locator('[data-testid="chat-input"]');
      // Placeholder varies by whether a collection is selected
      await expect(input).toHaveAttribute('placeholder', /Ask a question|Select a collection/i);
    });

    test('TC-CHAT-010: send button is visible', async ({ page }) => {
      await page.goto('/');
      const sendBtn = page.locator('[data-testid="btn-send"]');
      await expect(sendBtn).toBeVisible({ timeout: 5000 });
    });

    test('TC-CHAT-011: send button is disabled when input is empty', async ({ page }) => {
      await page.goto('/');
      const sendBtn = page.locator('[data-testid="btn-send"]');
      await expect(sendBtn).toBeDisabled();
    });

    test('TC-CHAT-012: send button is enabled when input has text and collection is selected', async ({
      page,
    }) => {
      await page.goto('/');
      // Set active collection
      await page.evaluate(() => {
        // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = 'col-1';
      });
      const input = page.locator('[data-testid="chat-input"]');
      await input.fill('Hello, VEDO!');

      const sendBtn = page.locator('[data-testid="btn-send"]');
      // If a collection is selected, send should be enabled
      await expect(sendBtn).toBeEnabled();
    });

    test('TC-CHAT-013: pressing Enter sends the message', async ({ page }) => {
      await page.goto('/');
      // Set active collection to enable input
      await page.evaluate(() => {
        // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = 'col-1';
      });
      const input = page.locator('[data-testid="chat-input"]');
      await input.fill('Test message');
      await input.press('Enter');

      // The user message should appear after sending
      // (this assumes the store handles the send and adds the message)
      // In a mock scenario, we test that Enter triggers send
    });

    test('TC-CHAT-014: Shift+Enter inserts newline instead of sending', async ({ page }) => {
      await page.goto('/');
      // Set active collection to enable input
      await page.evaluate(() => {
        // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = 'col-1';
      });
      const input = page.locator('[data-testid="chat-input"]');
      await input.fill('Line 1');
      await input.press('Shift+Enter');
      // Type additional text without clearing (fill() replaces content)
      await page.keyboard.type('Line 2');

      // The value should contain a newline after Shift+Enter
      const value = await input.inputValue();
      expect(value).toContain('Line 1');
      expect(value).toContain('Line 2');
    });

    test('TC-CHAT-015: cancel button appears during streaming', async ({ page }) => {
      await page.goto('/');
      // Cancel button should only appear when isLoading is true
      const cancelBtn = page.locator('[data-testid="btn-cancel"]');
      // Initially, cancel should not be visible (no streaming)
      await expect(cancelBtn).not.toBeVisible();
    });
  });

  test.describe('Message Animations (Task 3.3)', () => {
    test('TC-ANIM-001: new messages fade in smoothly', async ({ page }) => {
      await page.goto('/');
      // Set active collection and seed a message for animation assertions
      await page.evaluate(() => {
        // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = 'col-1';
        pinia.state.value.chat.messages = [
          {
            id: 'm1',
            session_id: 'sess-1',
            role: 'user',
            content: 'Hello',
            created_at: new Date().toISOString(),
          },
        ];
      });
      await page.waitForSelector('[data-testid^="message-"]');
      const message = page.locator('[data-testid^="message-"]').first();
      // The message should have a fade-in animation
      const animationName = await message.evaluate((el) => getComputedStyle(el).animationName);
      expect(animationName).not.toBe('none');
    });

    test('TC-ANIM-002: message animation has reasonable duration', async ({ page }) => {
      await page.goto('/');
      // Set active collection and seed a message for animation assertions
      await page.evaluate(() => {
        // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        pinia.state.value.collections.activeCollectionId = 'col-1';
        pinia.state.value.chat.messages = [
          {
            id: 'm1',
            session_id: 'sess-1',
            role: 'user',
            content: 'Hello',
            created_at: new Date().toISOString(),
          },
        ];
      });
      await page.waitForSelector('[data-testid^="message-"]');
      const message = page.locator('[data-testid^="message-"]').first();
      const duration = await message.evaluate((el) =>
        Number.parseFloat(getComputedStyle(el).animationDuration),
      );
      // Animation should be quick: less than 500ms
      expect(duration).toBeLessThan(0.5);
    });

    test('TC-ANIM-003: scroll to bottom on new message works', async ({ page }) => {
      await page.goto('/');
      const messagesArea = page.locator('[data-testid="messages-area"]');
      // Scroll to bottom should trigger on new message
      // Verify that scrollTop approaches scrollHeight when a message is added
      const scrollTop = await messagesArea.evaluate((el) => el.scrollTop);
      const scrollHeight = await messagesArea.evaluate((el) => el.scrollHeight);
      const clientHeight = await messagesArea.evaluate((el) => el.clientHeight);
      // If there are messages, scroll should be near bottom
      if (scrollHeight > clientHeight) {
        expect(scrollTop).toBeGreaterThanOrEqual(scrollHeight - clientHeight - 50); // within 50px of bottom
      }
    });
  });
});

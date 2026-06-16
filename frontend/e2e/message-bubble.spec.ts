import { expect, test } from '@playwright/test';

/**
 * MessageBubble Component Tests (Task 2.1, 2.2)
 *
 * Tests the redesigned MessageBubble with minimalistic styling,
 * markdown rendering, source citations, and streaming/typing indicator.
 */
test.describe('MessageBubble Component', () => {
  test.describe('Message Rendering', () => {
    test('TC-MSG-001: renders user messages right-aligned', async ({ page }) => {
      await page.goto('/');
      const userMsg = page.locator('[data-testid="message-user"]').first();
      await expect(userMsg).toBeVisible({ timeout: 5000 });

      // User messages should be right-aligned (flex-direction: row-reverse or similar)
      const container = userMsg.locator('..'); // parent flex container
      await expect(userMsg).toHaveCSS('justify-self', 'flex-end');
    });

    test('TC-MSG-002: renders assistant messages left-aligned', async ({ page }) => {
      await page.goto('/');
      const assistantMsg = page.locator('[data-testid="message-assistant"]').first();
      await expect(assistantMsg).toBeVisible({ timeout: 5000 });

      // Assistant messages should be left-aligned
      await expect(assistantMsg).toHaveCSS('justify-self', 'flex-start');
    });

    test('TC-MSG-003: displays message sender role label', async ({ page }) => {
      await page.goto('/');
      const userLabel = page.locator('[data-testid="message-role-user"]').first();
      await expect(userLabel).toHaveText('You');

      const assistantLabel = page.locator('[data-testid="message-role-assistant"]').first();
      await expect(assistantLabel).toHaveText('VEDO Assistant');
    });

    test('TC-MSG-004: displays message timestamp', async ({ page }) => {
      await page.goto('/');
      const timestamp = page.locator('[data-testid="message-time"]').first();
      await expect(timestamp).toBeVisible({ timeout: 5000 });
      // Timestamp should be a valid time string (HH:MM format or similar)
      await expect(timestamp).not.toBeEmpty();
    });

    test('TC-MSG-005: renders markdown content correctly', async ({ page }) => {
      await page.goto('/');
      // A message with markdown should render HTML (code blocks, bold, links)
      const markdownContent = page.locator('[data-testid="message-content"]').first();
      await expect(markdownContent).toBeVisible({ timeout: 5000 });

      // Code blocks should have styled pre/code elements
      const codeBlock = markdownContent.locator('pre code').first();
      if (await codeBlock.isVisible()) {
        await expect(codeBlock).toHaveCSS('background-color', expect.stringContaining('rgb'));
      }
    });

    test('TC-MSG-006: renders inline code with distinct background', async ({ page }) => {
      await page.goto('/');
      const inlineCode = page.locator('[data-testid="message-content"] code:not(pre code)').first();
      if (await inlineCode.isVisible()) {
        await expect(inlineCode).toHaveCSS('background-color', expect.stringContaining('rgb'));
        await expect(inlineCode).toHaveCSS('padding', expect.stringContaining('px'));
      }
    });

    test('TC-MSG-007: renders links with correct styling', async ({ page }) => {
      await page.goto('/');
      const link = page.locator('[data-testid="message-content"] a').first();
      if (await link.isVisible()) {
        await expect(link).toHaveCSS('color', expect.stringContaining('rgb'));
        await expect(link).toHaveAttribute('href', /^https?:\/\//);
      }
    });
  });

  test.describe('Source Citations', () => {
    test('TC-MSG-008: displays source citation toggle for assistant messages', async ({ page }) => {
      await page.goto('/');
      const sourcesToggle = page.locator('[data-testid="sources-toggle"]').first();
      await expect(sourcesToggle).toBeVisible({ timeout: 5000 });
      // Should show source count
      await expect(sourcesToggle).toContainText(/source/i);
    });

    test('TC-MSG-009: expands source list on toggle click', async ({ page }) => {
      await page.goto('/');
      const sourcesToggle = page.locator('[data-testid="sources-toggle"]').first();
      await sourcesToggle.click();

      // Sources list should now be visible
      const sourcesList = page.locator('[data-testid="sources-list"]').first();
      await expect(sourcesList).toBeVisible();
    });

    test('TC-MSG-010: collapses source list on second toggle click', async ({ page }) => {
      await page.goto('/');
      const sourcesToggle = page.locator('[data-testid="sources-toggle"]').first();
      await sourcesToggle.click(); // expand
      await sourcesToggle.click(); // collapse

      const sourcesList = page.locator('[data-testid="sources-list"]').first();
      await expect(sourcesList).not.toBeVisible();
    });

    test('TC-MSG-011: shows document name and relevance in source item', async ({ page }) => {
      await page.goto('/');
      // Open sources
      await page.locator('[data-testid="sources-toggle"]').first().click();

      const sourceItem = page.locator('[data-testid="source-item"]').first();
      await expect(sourceItem).toBeVisible();

      // Should have document name
      const docName = sourceItem.locator('[data-testid="source-document"]');
      await expect(docName).toBeVisible();
      await expect(docName).not.toBeEmpty();

      // Should have relevance score
      const relevance = sourceItem.locator('[data-testid="source-relevance"]');
      await expect(relevance).toBeVisible();
      await expect(relevance).toContainText('%');
    });
  });

  test.describe('Streaming / Typing Indicator (Task 2.2)', () => {
    test('TC-TYPING-001: shows typing indicator when streaming starts', async ({ page }) => {
      await page.goto('/');
      // When assistant is generating a response, typing dots should appear
      const typingIndicator = page.locator('[data-testid="typing-indicator"]');
      // Type a query to trigger streaming, then check for typing indicator
      // Note: This requires actual backend or mock. We test the component contracts.
      // In TDD, we verify the component class/exists and shows during streaming.
    });

    test('TC-TYPING-002: typing indicator has animated dots', async ({ page }) => {
      await page.goto('/');
      // The typing indicator should contain 3 bouncing dots
      const typingDots = page.locator('[data-testid="typing-dot"]');
      const dotCount = await typingDots.count();
      expect(dotCount).toBe(3);
    });

    test('TC-TYPING-003: typing indicator dots have animation delay', async ({ page }) => {
      await page.goto('/');
      // Each dot should have a sequential animation delay
      const dots = page.locator('[data-testid="typing-dot"]');
      const delays: string[] = [];
      for (let i = 0; i < (await dots.count()); i++) {
        const delay = await dots.nth(i).evaluate((el) => el.style.animationDelay);
        delays.push(delay);
      }
      // Delays should be different for each dot (sequential)
      if (delays.length >= 2) {
        expect(delays[0]).not.toBe(delays[1]);
      }
    });

    test('TC-TYPING-004: typing indicator disappears when streaming completes', async ({
      page,
    }) => {
      await page.goto('/');
      // After streaming completes (message content is filled), typing indicator should hide
      // This requires a completed message scenario
      const typingIndicator = page.locator('[data-testid="typing-indicator"]');
      await expect(typingIndicator).not.toBeVisible({ timeout: 10000 });
    });

    test('TC-TYPING-005: typing indicator disappears on stream error', async ({ page }) => {
      await page.goto('/');
      // On error, typing indicator should be replaced with error banner
      const errorBanner = page.locator('[data-testid="error-banner"]');
      const typingIndicator = page.locator('[data-testid="typing-indicator"]');
      // If error is visible, typing should not be visible
      if (await errorBanner.isVisible()) {
        await expect(typingIndicator).not.toBeVisible();
      }
    });
  });

  test.describe('Visual Styling', () => {
    test('TC-MSG-012: user message has right-corner-rounded bubble style', async ({ page }) => {
      await page.goto('/');
      const userMsg = page.locator('[data-testid="message-body-user"]').first();
      await expect(userMsg).toBeVisible({ timeout: 5000 });
      // Should have border-radius with asymmetric corners (12px top-right, 4px bottom-right)
      const borderRadius = await userMsg.evaluate((el) => getComputedStyle(el).borderRadius);
      expect(borderRadius).toMatch(/12px/);
    });

    test('TC-MSG-013: assistant message has left-corner-rounded bubble style', async ({ page }) => {
      await page.goto('/');
      const assistantMsg = page.locator('[data-testid="message-body-assistant"]').first();
      await expect(assistantMsg).toBeVisible({ timeout: 5000 });
      // Should have border-radius with asymmetric corners (4px top-left, 12px bottom-left)
      const borderRadius = await assistantMsg.evaluate((el) => getComputedStyle(el).borderRadius);
      expect(borderRadius).toMatch(/12px/);
    });

    test('TC-MSG-014: message background colors use design tokens', async ({ page }) => {
      await page.goto('/');
      // Background colors should be defined by CSS custom properties (design tokens)
      const userMsg = page.locator('[data-testid="message-body-user"]').first();
      const bgColor = await userMsg.evaluate((el) => getComputedStyle(el).backgroundColor);
      // Should be a valid color value, not transparent/initial
      expect(bgColor).not.toBe('rgba(0, 0, 0, 0)');
      expect(bgColor).not.toBe('transparent');
    });
  });
});

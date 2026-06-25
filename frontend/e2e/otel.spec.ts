import { expect, test } from '@playwright/test';

test.describe('OpenTelemetry logging integration', () => {
  test('frontend initializes without telemetry errors', async ({ page }) => {
    // Track console messages for errors
    const errors: string[] = [];
    page.on('pageerror', (err) => errors.push(err.message));
    page.on('console', (msg) => {
      if (msg.type() === 'error') {
        errors.push(msg.text());
      }
    });

    await page.goto('/');

    // Wait for app to load
    await page.waitForSelector('body');

    // Check no critical errors from telemetry initialization
    const telemetryErrors = errors.filter(
      (e) =>
        e.includes('telemetry') ||
        e.includes('opentelemetry') ||
        e.includes('OTLP') ||
        e.includes('LoggerProvider'),
    );
    expect(telemetryErrors).toEqual([]);
  });

  test('page renders without uncaught telemetry exceptions', async ({ page }) => {
    let uncaughtError: string | null = null;
    page.on('pageerror', (err) => {
      uncaughtError = err.message;
    });

    await page.goto('/');
    await page.waitForTimeout(2000);

    // Allow telemetry init to complete
    expect(uncaughtError).toBeNull();
  });
});

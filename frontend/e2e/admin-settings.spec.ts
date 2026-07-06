import { expect, test } from '@playwright/test';
import { API_URL, getTestAccessToken, setupAuth } from './helpers';

test.describe('Admin Settings: Model Configuration', () => {
  test.beforeEach(async ({ page }) => {
    await setupAuth(page);
  });

  test('TC-SETTINGS-001: settings tab loads with Models section', async ({ page }) => {
    await page.goto('/admin');

    // Wait for admin view to be visible
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });

    // Click the Settings tab
    await page.locator('[data-testid="admin-tab-settings"]').click();

    // Wait for settings panel to load
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    // Models section title should be visible
    await expect(page.locator('h3.section-title', { hasText: 'Models' })).toBeVisible();

    // All three model fields should be present
    await expect(page.locator('.setting-label', { hasText: 'LLM Model' })).toBeVisible();
    await expect(page.locator('.setting-label', { hasText: 'Embedding Model' })).toBeVisible();
    await expect(page.locator('.setting-label', { hasText: 'Rerank Model' })).toBeVisible();
  });

  test('TC-SETTINGS-002: default model values are displayed', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-settings"]').click();
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    // VSelect triggers show the currently selected value
    // LLM Model default: Claude Sonnet 4.6
    await expect(page.locator('.v-select__value', { hasText: 'Claude Sonnet 4.6' })).toBeVisible();

    // Embedding model default: all-MiniLM-L6-v2
    await expect(page.locator('.v-select__value', { hasText: 'all-MiniLM-L6-v2' })).toBeVisible();
  });

  test('TC-SETTINGS-003: change LLM model via dropdown and save', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-settings"]').click();
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    // Find the LLM Model VSelect trigger (the first VSelect in the Models section)
    const llmSection = page.locator('.settings-section').filter({ hasText: 'Models' });
    const llmTrigger = llmSection.locator('.v-select__trigger').first();

    // Open the dropdown
    await llmTrigger.click();
    await expect(page.locator('[data-testid="collection-select-dropdown"]')).toBeVisible();

    // Select "GPT 5.5" from the dropdown
    await page.locator('.v-select__option', { hasText: 'GPT 5.5' }).click();

    // The trigger should now show GPT 5.5
    await expect(llmTrigger).toContainText('GPT 5.5');

    // Click Save Changes
    await page.locator('button', { hasText: 'Save Changes' }).click();

    // Wait for success toast
    await expect(page.locator('.v-toast')).toContainText('Settings saved successfully', {
      timeout: 10000,
    });

    // Verify via API that the model was persisted
    const token = await getTestAccessToken();
    const settingsResp = await page.request.get(`${API_URL}/api/admin/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(settingsResp.status()).toBe(200);
    const settings = await settingsResp.json();
    expect(settings.llm_model).toBe('openai/gpt-5.5');
  });

  test('TC-SETTINGS-004: change embedding model and verify persistence', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-settings"]').click();
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    // Find the Embedding Model VSelect trigger
    const llmSection = page.locator('.settings-section').filter({ hasText: 'Models' });
    const embedTrigger = llmSection.locator('.v-select__trigger').nth(1);

    // Open the dropdown
    await embedTrigger.click();
    await expect(page.locator('[data-testid="collection-select-dropdown"]')).toBeVisible();

    // Select "BGE M3" from the dropdown
    await page.locator('.v-select__option', { hasText: 'BGE M3' }).click();

    // Verify the trigger updated
    await expect(embedTrigger).toContainText('BGE M3');

    // Save
    await page.locator('button', { hasText: 'Save Changes' }).click();
    await expect(page.locator('.v-toast')).toContainText('Settings saved successfully', {
      timeout: 10000,
    });

    // Verify via API
    const token = await getTestAccessToken();
    const settingsResp = await page.request.get(`${API_URL}/api/admin/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(settingsResp.status()).toBe(200);
    const settings = await settingsResp.json();
    expect(settings.embedding_model).toBe('baai/bge-m3');
  });

  test('TC-SETTINGS-005: change both LLM and embedding models at once', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-settings"]').click();
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    const modelsSection = page.locator('.settings-section').filter({ hasText: 'Models' });

    // Change LLM model to DeepSeek V4 Flash
    const llmTrigger = modelsSection.locator('.v-select__trigger').first();
    await llmTrigger.click();
    await page.locator('.v-select__option', { hasText: 'DeepSeek V4 Flash' }).click();

    // Change Embedding model to E5 Large V2
    const embedTrigger = modelsSection.locator('.v-select__trigger').nth(1);
    await embedTrigger.click();
    await page.locator('.v-select__option', { hasText: 'E5 Large V2' }).click();

    // Save both
    await page.locator('button', { hasText: 'Save Changes' }).click();
    await expect(page.locator('.v-toast')).toContainText('Settings saved successfully', {
      timeout: 10000,
    });

    // Verify both via API
    const token = await getTestAccessToken();
    const settingsResp = await page.request.get(`${API_URL}/api/admin/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(settingsResp.status()).toBe(200);
    const settings = await settingsResp.json();
    expect(settings.llm_model).toBe('deepseek/deepseek-v4-flash');
    expect(settings.embedding_model).toBe('intfloat/e5-large-v2');
  });

  test('TC-SETTINGS-006: all sections are visible', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-settings"]').click();
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    // All section titles should be present
    const sectionTitles = page.locator('h3.section-title');
    await expect(sectionTitles.filter({ hasText: 'Pipeline' })).toBeVisible();
    await expect(sectionTitles.filter({ hasText: 'Chunking' })).toBeVisible();
    await expect(sectionTitles.filter({ hasText: 'Search' })).toBeVisible();
    await expect(sectionTitles.filter({ hasText: 'Models' })).toBeVisible();
  });

  test('TC-SETTINGS-007: API returns all settings with expected keys', async ({ request }) => {
    const token = await getTestAccessToken();

    const response = await request.get(`${API_URL}/api/admin/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(response.status()).toBe(200);

    const body = (await response.json()) as Record<string, unknown>;

    // Verify all expected keys exist
    const expectedKeys = [
      'advanced_rag_enabled',
      'multi_query_enabled',
      'hyde_enabled',
      'bm25_enabled',
      'reranking_enabled',
      'chunk_method',
      'chunk_size',
      'chunk_overlap',
      'hybrid_top_k',
      'rerank_top_k',
      'multi_query_count',
      'llm_model',
      'llm_rerank_model',
      'embedding_model',
      'llm_max_history_messages',
      'llm_context_token_budget',
    ];

    for (const key of expectedKeys) {
      expect(body).toHaveProperty(key);
    }

    // Verify types
    expect(typeof body.llm_model).toBe('string');
    expect(typeof body.embedding_model).toBe('string');
    expect(typeof body.chunk_size).toBe('number');
    expect(typeof body.advanced_rag_enabled).toBe('boolean');
  });

  test('TC-SETTINGS-008: modify rerank model via text input and save', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-settings"]').click();
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    // Find the Rerank Model input (it's a VInput, the 3rd field in Models section)
    const modelsSection = page.locator('.settings-section').filter({ hasText: 'Models' });
    const rerankInput = modelsSection.locator('input').first();

    // Clear and type a new value
    await rerankInput.fill('');
    await rerankInput.fill('anthropic/claude-sonnet-5');

    // Save
    await page.locator('button', { hasText: 'Save Changes' }).click();
    await expect(page.locator('.v-toast')).toContainText('Settings saved successfully', {
      timeout: 10000,
    });

    // Verify via API
    const token = await getTestAccessToken();
    const settingsResp = await page.request.get(`${API_URL}/api/admin/settings`, {
      headers: { Authorization: `Bearer ${token}` },
    });
    expect(settingsResp.status()).toBe(200);
    const settings = await settingsResp.json();
    expect(settings.llm_rerank_model).toBe('anthropic/claude-sonnet-5');
  });

  test('TC-SETTINGS-009: Reset to Defaults resets form values', async ({ page }) => {
    await page.goto('/admin');
    await page.locator('[data-testid="admin-tab-settings"]').click();
    await expect(page.locator('[data-testid="settings-panel"]')).toBeVisible({
      timeout: 10000,
    });

    // First change a model to something custom
    const modelsSection = page.locator('.settings-section').filter({ hasText: 'Models' });
    const llmTrigger = modelsSection.locator('.v-select__trigger').first();
    await llmTrigger.click();
    await page.locator('.v-select__option', { hasText: 'GPT 5.5' }).click();
    await expect(llmTrigger).toContainText('GPT 5.5');

    // Click Reset to Defaults
    await page.locator('button', { hasText: 'Reset to Defaults' }).click();

    // Reset dialog should appear
    await expect(page.locator('.v-overlay__dialog')).toBeVisible({ timeout: 5000 });

    // Click Reset in the dialog
    await page.locator('.v-overlay__dialog button', { hasText: 'Reset' }).click();

    // The form should be reset but not yet saved — the trigger should still show old value
    // because the form was reset in memory; Click Save to persist default
    // Actually the form value changes back to default in memory, so VSelect shows the default
    await expect(llmTrigger).toContainText('Claude Sonnet 4.6');
  });
});

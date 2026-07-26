import { expect, test } from '@playwright/test';
import {
  API_URL,
  apiRequest,
  fileInput,
  getTestAccessToken,
  setActiveCollection,
  setupAuthAndCollection,
} from './helpers';

test.describe('Document lifecycle with real backend', () => {
  test('TC-REINDEX-001: upload document then query returns backend response', async ({
    page,
    request,
  }) => {
    test.setTimeout(60_000);
    const collection = await setupAuthAndCollection(page, request, `Docs Query ${Date.now()}`);

    // Reset backend settings to defaults to avoid state leakage from
    // admin-settings tests (which change LLM/embedding/rerank models).
    const token = await getTestAccessToken();
    await request.fetch(`${API_URL}/api/admin/settings`, {
      method: 'POST',
      headers: {
        Authorization: `Bearer ${token}`,
        'Content-Type': 'application/json',
      },
      data: {
        advanced_rag_enabled: true,
        multi_query_enabled: true,
        hyde_enabled: true,
        bm25_enabled: true,
        reranking_enabled: true,
        chunk_method: 'paragraph',
        chunk_size: 1000,
        chunk_overlap: 200,
        hybrid_top_k: 20,
        rerank_top_k: 5,
        multi_query_count: 3,
        llm_model: 'anthropic/claude-sonnet-4.6',
        llm_rerank_model: 'anthropic/claude-sonnet-4.6',
        embedding_model: 'sentence-transformers/all-minilm-l6-v2',
        llm_max_history_messages: 20,
        llm_context_token_budget: 6000,
      },
    });

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);
    await expect(page.locator('.document-list')).toBeVisible({
      timeout: 10000,
    });
    await fileInput(page).setInputFiles({
      name: 'config-guide.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from(
        '# Configuration Guide\n\nRate limiting is configured by backend middleware.',
      ),
    });
    await expect(page.locator('.dl-item__name').first()).toContainText('config-guide.md', {
      timeout: 30000,
    });

    await page.goto('/');
    await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);
    await page.locator('[data-testid="chat-input"]').fill('What is configured?');
    await page.locator('[data-testid="btn-send"]').click();

    await expect(page.locator('[data-testid="message-assistant"]').first()).toBeVisible({
      timeout: 60000,
    });
  });

  test('TC-REINDEX-003: deleted document disappears from real backend document list', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `Docs Delete ${Date.now()}`);

    await page.goto('/admin');
    await setActiveCollection(page, collection.id);
    await expect(page.locator('.document-list')).toBeVisible({
      timeout: 10000,
    });
    await fileInput(page).setInputFiles({
      name: 'delete-me.md',
      mimeType: 'text/markdown',
      buffer: Buffer.from('# Delete Me\n\nContent to be deleted.'),
    });
    await expect(page.locator('.dl-item__name').first()).toContainText('delete-me.md', {
      timeout: 30000,
    });

    const documents = await apiRequest<Array<{ id: string; name: string }>>(
      request,
      'GET',
      `/api/documents?collection_id=${collection.id}`,
    );
    const document = documents.find((item) => item.name === 'delete-me.md');
    expect(document?.id).toBeTruthy();

    await apiRequest(request, 'DELETE', `/api/documents/${document?.id}`);
    const remaining = await apiRequest<Array<{ id: string; name: string }>>(
      request,
      'GET',
      `/api/documents?collection_id=${collection.id}`,
    );
    expect(remaining.some((item) => item.id === document?.id)).toBe(false);
  });
});

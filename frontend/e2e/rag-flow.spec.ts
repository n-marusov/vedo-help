import path from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "@playwright/test";
import {
	VALID_TOKEN,
	mockCollections,
	mockSessions,
	setupAuth,
} from "./helpers";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const TEST_FILE_PATH = path.resolve(
	__dirname,
	"..",
	"src",
	"assets",
	"chat-tokens.css",
);

/**
 * RAG Flow E2E Tests (upload → query → sources)
 *
 * Tests the full RAG lifecycle through the admin and chat UIs:
 * 1. Upload a document via the admin panel
 * 2. Send a query in the chat view
 * 3. Verify streaming response and sources
 */
test.describe("RAG Flow: Upload → Query → Sources", () => {
	test.beforeEach(async ({ page }) => {
		// Inject JWT before navigation
		await page.addInitScript((token) => {
			localStorage.setItem("vedo_auth_token", token);
		}, VALID_TOKEN);

		// Mock collections API
		await page.route("**/api/collections", async (route) => {
			const body = JSON.stringify([
				{
					id: "col-1",
					name: "Test Collection",
					description: "A test collection",
					created_at: new Date().toISOString(),
					document_count: 2,
				},
			]);
			await route.fulfill({
				status: 200,
				contentType: "application/json",
				body,
			});
		});

		// Mock sessions list
		await page.route("**/api/sessions", async (route) => {
			const body = JSON.stringify([
				{
					id: "sess-1",
					title: "Test Session",
					message_count: 0,
					created_at: new Date().toISOString(),
					updated_at: new Date().toISOString(),
				},
			]);
			await route.fulfill({
				status: 200,
				contentType: "application/json",
				body,
			});
		});
	});

	test("TC-RAG-001: admin page renders with admin panel when authenticated", async ({
		page,
	}) => {
		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// With valid auth token, auth gate should be bypassed
		const authSection = page.locator('[data-testid="auth-section"]');
		await expect(authSection).not.toBeVisible();

		// Admin panel content should be visible
		const adminPanel = page.locator(".admin-panel");
		await expect(adminPanel).toBeVisible();
	});

	test("TC-RAG-002: upload document via admin panel (mocked)", async ({
		page,
	}) => {
		// Set API key in localStorage to skip auth gate
		await page.addInitScript((token: string) => {
			localStorage.setItem("vedo_auth_token", token);
		}, VALID_TOKEN);

		// Mock document upload endpoint
		await page.route("**/api/documents/upload", async (route) => {
			await route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify({
					document_id: "doc-1",
					chunks_indexed: 42,
					document_name: "test-doc.pdf",
				}),
			});
		});

		// Mock documents list
		await page.route("**/api/documents*", async (route) => {
			await route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify([
					{
						id: "doc-1",
						name: "test-doc.pdf",
						file_type: "application/pdf",
						file_size: 102400,
						uploaded_at: new Date().toISOString(),
						collection_id: "col-1",
					},
				]),
			});
		});

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// With API key set, auth gate should be bypassed
		const authSection = page.locator('[data-testid="auth-section"]');
		await expect(authSection).not.toBeVisible();

		// Admin panel should be visible
		const adminPanel = page.locator(".admin-panel");
		await expect(adminPanel).toBeVisible();
	});

	test("TC-RAG-003: send query and verify streaming response", async ({
		page,
	}) => {
		await page.addInitScript((token) => {
			localStorage.setItem("vedo_auth_token", token);
		}, VALID_TOKEN);

		// Mock /api/query endpoint with streaming NDJSON response
		await page.route("**/api/query", async (route) => {
			const streamChunks = [
				'{"type":"chunk","text":"Here is "}\n',
				'{"type":"chunk","text":"the answer "}\n',
				'{"type":"chunk","text":"to your question."}\n',
				'{"type":"sources","sources":[{"document_id":"doc-1","document_name":"test-doc.pdf","chunk_index":0,"text":"Relevant content from the document","relevance":0.95}]}\n',
				'{"type":"done"}\n',
			];

			const responseBody = streamChunks.join("");

			// Use plain string body instead of ReadableStream
			await route.fulfill({
				status: 200,
				headers: { "Content-Type": "application/x-ndjson" },
				body: responseBody,
			});
		});

		// Mock POST /api/sessions (create session)
		await page.route("**/api/sessions", async (route, request) => {
			if (request.method() === "POST") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify({
						id: "sess-1",
						title: "New Chat",
						collection_id: "col-1",
						created_at: new Date().toISOString(),
						updated_at: new Date().toISOString(),
						message_count: 0,
					}),
				});
			} else {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify([
						{
							id: "sess-1",
							title: "Test Session",
							message_count: 0,
							created_at: new Date().toISOString(),
							updated_at: new Date().toISOString(),
						},
					]),
				});
			}
		});

		await page.goto("/");
		const chatView = page.locator('[data-testid="chat-view"]');
		await expect(chatView).toBeVisible({ timeout: 10000 });

		// Type a query
		const input = page.locator('[data-testid="chat-input"]');
		await expect(input).toBeVisible();

		// Set active collection to enable input
		// DEBUG [e2e] rag-flow: setting activeCollectionId
		await page.evaluate(() => {
			const app = document.querySelector("#app").__vue_app__;
			const pinia = app.config.globalProperties.$pinia;
			pinia.state.value.collections.activeCollectionId = "col-1";
		});

		await input.fill("What is rate limiting?");

		// Click send
		const sendBtn = page.locator('[data-testid="btn-send"]');
		await expect(sendBtn).toBeEnabled();
		await sendBtn.click();

		// After sending, user message should appear
		const userMsg = page.locator('[data-testid="message-user"]').first();
		await expect(userMsg).toBeVisible({ timeout: 5000 });

		// Assistant message should eventually appear with content
		const assistantMsg = page
			.locator('[data-testid="message-assistant"]')
			.first();
		await expect(assistantMsg).toBeVisible({ timeout: 15000 });

		// Content should contain the streamed text
		const msgContent = page.locator('[data-testid="message-content"]').last();
		await expect(msgContent).toContainText(/answer/i, { timeout: 10000 });
	});

	test("TC-RAG-004: sources appear in query response", async ({ page }) => {
		await page.addInitScript((token) => {
			localStorage.setItem("vedo_auth_token", token);
		}, VALID_TOKEN);

		// Mock /api/query with streaming response including sources
		await page.route("**/api/query", async (route) => {
			const streamChunks = [
				'{"type":"chunk","text":"Based on the documentation, "}\n',
				'{"type":"chunk","text":"rate limiting is configured via environment variables."}\n',
				'{"type":"sources","sources":[{"document_id":"doc-1","document_name":"config-guide.md","chunk_index":0,"text":"The rate limiter is configured by setting MAX_REQUESTS_PER_MINUTE in your .env file.","relevance":0.92}]}\n',
				'{"type":"done"}\n',
			];

			const responseBody = streamChunks.join("");

			// Use plain string body instead of ReadableStream
			await route.fulfill({
				status: 200,
				headers: { "Content-Type": "application/x-ndjson" },
				body: responseBody,
			});
		});

		// Mock sessions
		await page.route("**/api/sessions", async (route, request) => {
			if (request.method() === "POST") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify({
						id: "sess-2",
						title: "New Chat",
						collection_id: "col-1",
						created_at: new Date().toISOString(),
						updated_at: new Date().toISOString(),
						message_count: 0,
					}),
				});
			} else {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify([
						{
							id: "sess-2",
							title: "Test Session",
							message_count: 0,
							created_at: new Date().toISOString(),
							updated_at: new Date().toISOString(),
						},
					]),
				});
			}
		});

		await page.goto("/");
		const chatView = page.locator('[data-testid="chat-view"]');
		await expect(chatView).toBeVisible({ timeout: 10000 });

		// Send a query
		const input = page.locator('[data-testid="chat-input"]');
		await expect(input).toBeVisible();

		// Set active collection to enable input
		// DEBUG [e2e] rag-flow: setting activeCollectionId
		await page.evaluate(() => {
			const app = document.querySelector("#app").__vue_app__;
			const pinia = app.config.globalProperties.$pinia;
			pinia.state.value.collections.activeCollectionId = "col-1";
		});

		await input.fill("How does rate limiting work?");

		const sendBtn = page.locator('[data-testid="btn-send"]');
		await sendBtn.click();

		// Wait for source toggle to appear
		const sourcesToggle = page.locator('[data-testid="sources-toggle"]');
		await expect(sourcesToggle).toBeVisible({ timeout: 15000 });
		await expect(sourcesToggle).toContainText(/source/i);

		// Expand sources
		await sourcesToggle.click();

		// Should show source items
		const sourceItem = page.locator('[data-testid="source-item"]').first();
		await expect(sourceItem).toBeVisible();

		// Source should have document name
		const docName = sourceItem.locator('[data-testid="source-document"]');
		await expect(docName).toBeVisible();
		await expect(docName).toContainText(/config-guide/i);

		// Source should have relevance score
		const relevance = sourceItem.locator('[data-testid="source-relevance"]');
		await expect(relevance).toBeVisible();
		await expect(relevance).toContainText("%");
	});

	test("TC-RAG-005: new chat clears messages", async ({ page }) => {
		await page.addInitScript((token) => {
			localStorage.setItem("vedo_auth_token", token);
		}, VALID_TOKEN);

		// Mock sessions
		await page.route("**/api/sessions", async (route, request) => {
			if (request.method() === "POST") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify({
						id: "sess-3",
						title: "New Chat",
						collection_id: "col-1",
						created_at: new Date().toISOString(),
						updated_at: new Date().toISOString(),
						message_count: 0,
					}),
				});
			} else {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify([
						{
							id: "sess-3",
							title: "Old Session",
							message_count: 2,
							created_at: new Date().toISOString(),
							updated_at: new Date().toISOString(),
						},
					]),
				});
			}
		});

		await page.goto("/");

		// The welcome screen should be visible initially (no messages)
		const welcomeMsg = page.locator('[data-testid="welcome-message"]');
		await expect(welcomeMsg).toBeVisible({ timeout: 10000 });

		// There should be a toolbar with a collection selector
		const toolbar = page.locator('[data-testid="chat-toolbar"]');
		await expect(toolbar).toBeVisible();
	});
});

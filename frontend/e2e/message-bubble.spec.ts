import {
	type APIRequestContext,
	type Page,
	expect,
	test,
} from "@playwright/test";
import { setupAuthAndCollection } from "./helpers";

/**
 * Helper: seed chat store with messages via Pinia.
 */
async function seedMessages(page: Page, msgs: Record<string, unknown>[]) {
	await page.evaluate((messages) => {
		// biome-ignore lint/suspicious/noExplicitAny: E2E test helper needs access to Vue internals
		const app = (document.querySelector("#app") as any).__vue_app__;
		const pinia = app.config.globalProperties.$pinia;
		const state = pinia.state.value.chat;
		state.messages = messages;
		state.activeSessionId = "sess-1";
	}, msgs);
}

/**
 * Helper: set up real auth/backend collection and navigate to /.
 */
async function setupChatPage(page: Page, request: APIRequestContext) {
	await setupAuthAndCollection(page, request, `MessageBubble ${Date.now()}`);

	await page.goto("/");
	await expect(page.locator('[data-testid="chat-view"]')).toBeVisible({
		timeout: 10000,
	});
}

/**
 * MessageBubble Component Tests
 */
test.describe("MessageBubble Component", () => {
	test.describe("Message Rendering", () => {
		test("TC-MSG-001: renders user messages right-aligned", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			// Inject messages into Pinia store so we can observe them
			await seedMessages(page, [
				{
					id: "u1",
					session_id: "sess-1",
					role: "user",
					content: "Hello",
					created_at: new Date().toISOString(),
				},
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Hi there",
					created_at: new Date().toISOString(),
				},
			]);
			// Wait for the page to re-render
			await expect(
				page.locator('[data-testid="message-user"]').first(),
			).toBeVisible({
				timeout: 5000,
			});

			const userMsg = page.locator('[data-testid="message-user"]').first();
			await expect(userMsg).toHaveCSS("align-self", "flex-end");
		});

		test("TC-MSG-002: renders assistant messages left-aligned", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "u1",
					session_id: "sess-1",
					role: "user",
					content: "Hello",
					created_at: new Date().toISOString(),
				},
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Hi there",
					created_at: new Date().toISOString(),
				},
			]);
			const assistantMsg = page
				.locator('[data-testid="message-assistant"]')
				.first();
			await expect(assistantMsg).toBeVisible({ timeout: 5000 });

			await expect(assistantMsg).toHaveCSS("align-self", "flex-start");
		});

		test("TC-MSG-003: identifies user message by data-testid role", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "u1",
					session_id: "sess-1",
					role: "user",
					content: "Hello",
					created_at: new Date().toISOString(),
				},
			]);
			// MessageBubble uses data-testid="message-user" for user role and
			// data-testid="message-body-user" for content wrapper
			const userMsg = page.locator('[data-testid="message-user"]').first();
			await expect(userMsg).toBeVisible({ timeout: 5000 });
		});

		test("TC-MSG-004: displays message timestamp", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "u1",
					session_id: "sess-1",
					role: "user",
					content: "Hello",
					created_at: new Date().toISOString(),
				},
			]);
			const timestamp = page.locator('[data-testid="message-time"]').first();
			await expect(timestamp).toBeVisible({ timeout: 5000 });
			await expect(timestamp).not.toBeEmpty();
		});

		test("TC-MSG-005: renders markdown content correctly", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content:
						'Code: `let x = 1`\n\n```python\nprint("hello")\n```\n\n**bold**',
					created_at: new Date().toISOString(),
				},
			]);
			const markdownContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(markdownContent).toBeVisible({ timeout: 5000 });

			// Code blocks should have styled pre/code elements
			const codeBlock = markdownContent.locator("pre code").first();
			await expect(codeBlock).toBeVisible();
		});

		test("TC-MSG-006: renders inline code with distinct background", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Use `inline code` here",
					created_at: new Date().toISOString(),
				},
			]);
			const inlineCode = page
				.locator('[data-testid="message-content"] code:not(pre code)')
				.first();
			await expect(inlineCode).toBeVisible();
			await expect(inlineCode).toHaveCSS("background-color", /rgb/i);
			// DEBUG [e2e] message-bubble: CSS shorthand → longhand assertion
			await expect(inlineCode).toHaveCSS("padding-left", /px/i);
		});

		test("TC-MSG-007: renders links with correct styling", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Visit https://example.com for info",
					created_at: new Date().toISOString(),
				},
			]);
			const link = page.locator('[data-testid="message-content"] a').first();
			await expect(link).toBeVisible();
			await expect(link).toHaveCSS("color", /rgb/i);
		});
	});

	test.describe("Source Citations", () => {
		test("TC-MSG-008: displays source citation toggle for assistant messages", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Answer with sources",
					sources: JSON.stringify([
						{
							document_id: "doc-1",
							document_name: "test.pdf",
							chunk_index: 0,
							text: "content",
							relevance: 0.95,
						},
					]),
					created_at: new Date().toISOString(),
				},
			]);
			const sourcesToggle = page
				.locator('[data-testid="sources-toggle"]')
				.first();
			await expect(sourcesToggle).toBeVisible({ timeout: 5000 });
			await expect(sourcesToggle).toContainText(/source/i);
		});

		test("TC-MSG-009: expands source list on toggle click", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Answer",
					sources: JSON.stringify([
						{
							document_id: "doc-1",
							document_name: "test.pdf",
							chunk_index: 0,
							text: "content",
							relevance: 0.95,
						},
					]),
					created_at: new Date().toISOString(),
				},
			]);
			const sourcesToggle = page
				.locator('[data-testid="sources-toggle"]')
				.first();
			await sourcesToggle.click();

			const sourcesList = page.locator('[data-testid="sources-list"]').first();
			await expect(sourcesList).toBeVisible();
		});

		test("TC-MSG-010: collapses source list on second toggle click", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Answer",
					sources: JSON.stringify([
						{
							document_id: "doc-1",
							document_name: "test.pdf",
							chunk_index: 0,
							text: "content",
							relevance: 0.95,
						},
					]),
					created_at: new Date().toISOString(),
				},
			]);
			const sourcesToggle = page
				.locator('[data-testid="sources-toggle"]')
				.first();
			await sourcesToggle.click(); // expand
			await sourcesToggle.click(); // collapse

			const sourcesList = page.locator('[data-testid="sources-list"]').first();
			await expect(sourcesList).not.toBeVisible();
		});

		test("TC-MSG-011: shows document name and relevance in source item", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Answer",
					sources: JSON.stringify([
						{
							document_id: "doc-1",
							document_name: "test.pdf",
							chunk_index: 0,
							text: "content",
							relevance: 0.95,
						},
					]),
					created_at: new Date().toISOString(),
				},
			]);
			await page.locator('[data-testid="sources-toggle"]').first().click();

			const sourceItem = page.locator('[data-testid="source-item"]').first();
			await expect(sourceItem).toBeVisible();

			const docName = sourceItem.locator('[data-testid="source-document"]');
			await expect(docName).toBeVisible();
			await expect(docName).not.toBeEmpty();

			const relevance = sourceItem.locator('[data-testid="source-relevance"]');
			await expect(relevance).toBeVisible();
			await expect(relevance).toContainText("%");
		});
	});

	test.describe("Visual Styling", () => {
		test("TC-MSG-012: user message has right-corner-rounded bubble style", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "u1",
					session_id: "sess-1",
					role: "user",
					content: "Hello",
					created_at: new Date().toISOString(),
				},
			]);
			const userMsg = page.locator('[data-testid="message-body-user"]').first();
			await expect(userMsg).toBeVisible({ timeout: 5000 });
			const borderRadius = await userMsg.evaluate(
				(el) => getComputedStyle(el).borderRadius,
			);
			expect(borderRadius).toMatch(/18px/);
		});

		test("TC-MSG-013: assistant message has left-corner-rounded bubble style", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Hi",
					created_at: new Date().toISOString(),
				},
			]);
			const assistantMsg = page
				.locator('[data-testid="message-body-assistant"]')
				.first();
			await expect(assistantMsg).toBeVisible({ timeout: 5000 });
			const borderRadius = await assistantMsg.evaluate(
				(el) => getComputedStyle(el).borderRadius,
			);
			expect(borderRadius).toMatch(/18px/);
		});

		test("TC-MSG-014: message background colors use design tokens", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "u1",
					session_id: "sess-1",
					role: "user",
					content: "Hello",
					created_at: new Date().toISOString(),
				},
			]);
			const userMsg = page.locator('[data-testid="message-body-user"]').first();
			const bgColor = await userMsg.evaluate(
				(el) => getComputedStyle(el).backgroundColor,
			);
			expect(bgColor).not.toBe("rgba(0, 0, 0, 0)");
			expect(bgColor).not.toBe("transparent");
		});
	});

	// ===========================================================================
	// CODE BLOCK & SYNTAX HIGHLIGHTING E2E TESTS (Task 5)
	// ===========================================================================
	test.describe("Code Block Rendering", () => {
		test("TC-CODE-001: code block renders with syntax highlighting classes (hljs)", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: '```python\nprint("hello world")\n```',
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			// The rendered code block should have highlight.js classes
			const highlightedCode = msgContent.locator(".hljs");
			await expect(highlightedCode).toBeVisible();

			// The code should be inside a code-block-wrapper
			const wrapper = msgContent.locator(".code-block-wrapper");
			await expect(wrapper).toBeVisible();
		});

		test("TC-CODE-002: code block has language label", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "```rust\nfn main() {}\n```",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			// Language label should be visible in the code block header
			const langLabel = msgContent.locator(".code-lang-label");
			await expect(langLabel).toBeVisible();
			await expect(langLabel).toHaveText(/rust/i);
		});

		test("TC-CODE-003: copy button is visible on code blocks", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "```javascript\nconst x = 1;\n```",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			const copyBtn = msgContent.locator(".copy-code-btn");
			await expect(copyBtn).toBeVisible();
			await expect(copyBtn).toHaveText("Copy");
		});

		test("TC-CODE-004: copy button copies content to clipboard", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "```javascript\nconst x = 42;\nconsole.log(x);\n```",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			// Grant clipboard permissions
			await page
				.context()
				.grantPermissions(["clipboard-read", "clipboard-write"]);

			const copyBtn = msgContent.locator(".copy-code-btn");
			await copyBtn.click();

			// Read from clipboard and verify
			const clipboardText = await page.evaluate(() =>
				navigator.clipboard.readText(),
			);
			expect(clipboardText).toContain("const x = 42;");
			expect(clipboardText).toContain("console.log(x);");
		});

		test('TC-CODE-005: copy button shows "Copied!" state after click', async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: '```bash\necho "test"\n```',
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			await page
				.context()
				.grantPermissions(["clipboard-read", "clipboard-write"]);

			const copyBtn = msgContent.locator(".copy-code-btn");
			await copyBtn.click();

			// After click, button text should change to "Copied!"
			await expect(copyBtn).toHaveText("Copied!");
		});
	});

	// ===========================================================================
	// MARKDOWN FEATURES E2E TESTS (Task 5)
	// ===========================================================================
	test.describe("Markdown Features", () => {
		test("TC-MD-001: table renders with correct HTML structure", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content:
						"| Name  | Age |\n|-------|-----|\n| Alice | 30  |\n| Bob   | 25  |",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			// Table should be rendered
			const table = msgContent.locator("table");
			await expect(table).toBeVisible();

			// Should have thead and tbody
			const thead = table.locator("thead");
			await expect(thead).toBeVisible();
			await expect(thead.locator("th").first()).toHaveText("Name");

			const tbody = table.locator("tbody");
			await expect(tbody).toBeVisible();
			await expect(tbody.locator("tr")).toHaveCount(2);
		});

		test("TC-MD-002: blockquote renders with correct styling", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "> This is a blockquote\n> With multiple lines",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			const blockquote = msgContent.locator("blockquote");
			await expect(blockquote).toBeVisible();
			await expect(blockquote).toContainText("blockquote");
		});

		test("TC-MD-003: inline code has distinct background", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Use the `renderMarkdown()` function to parse content.",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			// Inline code (code not inside pre) should have distinct background
			const inlineCode = msgContent.locator("code:not(pre code)").first();
			await expect(inlineCode).toBeVisible();
			await expect(inlineCode).toHaveText(/renderMarkdown\(\)/);

			// Should have a background color (not transparent)
			const bgColor = await inlineCode.evaluate(
				(el) => getComputedStyle(el).backgroundColor,
			);
			expect(bgColor).not.toBe("rgba(0, 0, 0, 0)");
			expect(bgColor).not.toBe("transparent");
		});

		test("TC-MD-004: list items render correctly", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "- Item one\n- Item two\n- Item three",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			const list = msgContent.locator("ul");
			await expect(list).toBeVisible();
			await expect(list.locator("li")).toHaveCount(3);
		});

		test("TC-MD-005: ordered list renders correctly", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "1. First\n2. Second\n3. Third",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			const list = msgContent.locator("ol");
			await expect(list).toBeVisible();
			await expect(list.locator("li")).toHaveCount(3);
		});

		test("TC-MD-006: headings render with correct hierarchy", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "# H1 Title\n\n## H2 Section\n\n### H3 Subsection",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			await expect(msgContent.locator("h1")).toHaveText("H1 Title");
			await expect(msgContent.locator("h2")).toHaveText("H2 Section");
			await expect(msgContent.locator("h3")).toHaveText("H3 Subsection");
		});

		test("TC-MD-007: horizontal rule renders as styled element", async ({
			page,
			request,
		}) => {
			await setupChatPage(page, request);
			await seedMessages(page, [
				{
					id: "a1",
					session_id: "sess-1",
					role: "assistant",
					content: "Above\n\n---\n\nBelow",
					created_at: new Date().toISOString(),
				},
			]);

			const msgContent = page
				.locator('[data-testid="message-content"]')
				.first();
			await expect(msgContent).toBeVisible({ timeout: 5000 });

			const hr = msgContent.locator("hr");
			await expect(hr).toBeVisible();

			// hr should have a background color (styled via CSS, design tokens)
			// DEBUG [e2e] message-bubble: CSS shorthand → longhand assertion
			const bgColor = await hr.evaluate(
				(el) => getComputedStyle(el).backgroundColor,
			);
			expect(bgColor).toBeTruthy();
			expect(bgColor).not.toBe("rgba(0, 0, 0, 0)");
			expect(bgColor).not.toBe("transparent");
		});
	});
});

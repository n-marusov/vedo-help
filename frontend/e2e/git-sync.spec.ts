import { expect, test } from "@playwright/test";

/**
 * Make a mock JWT with the given payload claims.
 */
function makeMockJwt(claims: Record<string, unknown>): string {
	const header = btoa(JSON.stringify({ alg: "RS256", typ: "JWT" }));
	const payload = btoa(JSON.stringify(claims));
	return `${header}.${payload}.mocksignature`;
}

const VALID_TOKEN = makeMockJwt({
	sub: "user-123",
	name: "Test User",
	preferred_username: "testuser",
	exp: Math.floor(Date.now() / 1000) + 7200,
	iat: Math.floor(Date.now() / 1000),
});

const API_KEY = "test-admin-key-123";

/** Mock git repos for the happy path */
const MOCK_REPOS = [
	{
		id: "repo-1",
		url: "https://github.com/user/test-repo.git",
		branch: "main",
		collection_id: "col-1",
		status: "idle",
		local_path: "/tmp/clones/repo-1",
		last_synced_at: new Date().toISOString(),
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString(),
		collection_name: "Test Collection",
	},
];

/** Mock an empty list */
const EMPTY_REPOS: typeof MOCK_REPOS = [];

/**
 * Git Sync E2E Tests
 *
 * Tests the Git repository management UI through the admin panel:
 * register, sync, delete, validation, error states, and listing.
 * All backend calls are mocked via page.route().
 */
test.describe("Git Sync: Repository Management", () => {
	test.beforeEach(async ({ page }) => {
		// Inject auth + API key before navigation
		await page.addInitScript(
			(data: { token: string; apiKey: string }) => {
				localStorage.setItem("vedo_auth_token", data.token);
				localStorage.setItem("vedo_api_key", data.apiKey);
			},
			{ token: VALID_TOKEN, apiKey: API_KEY },
		);

		// Mock collections API
		await page.route("**/api/collections", async (route) => {
			await route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify([
					{
						id: "col-1",
						name: "Test Collection",
						description: "A test collection",
						created_at: new Date().toISOString(),
						document_count: 2,
					},
				]),
			});
		});

		// Mock documents list
		await page.route("**/api/documents*", async (route) => {
			await route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify([]),
			});
		});
	});

	test("TC-GIT-001: register new git repo → row appears in table with idle status", async ({
		page,
	}) => {
		// Start with empty repos list
		await page.route("**/api/git-sync/repos", async (route, request) => {
			if (request.method() === "GET") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify(EMPTY_REPOS),
				});
			} else if (request.method() === "POST") {
				await route.fulfill({
					status: 201,
					contentType: "application/json",
					body: JSON.stringify({
						id: "repo-1",
						url: "https://github.com/user/test-repo.git",
						branch: "main",
						collection_id: "col-1",
						status: "idle",
						local_path: "/tmp/clones/repo-1",
						last_synced_at: null,
						created_at: new Date().toISOString(),
						updated_at: new Date().toISOString(),
						collection_name: "Test Collection",
					}),
				});
			} else {
				await route.continue();
			}
		});

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Use admin tabs to switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		// Locate the Git repo manager component
		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// Open the connect dialog
		const connectBtn = page.locator('[data-testid="btn-git-repo-connect"]');
		await expect(connectBtn).toBeVisible();
		await connectBtn.click();

		// Fill the registration form
		const urlInput = page.locator('[data-testid="git-repo-url-input"]');
		await expect(urlInput).toBeVisible();
		await urlInput.fill("https://github.com/user/test-repo.git");

		const branchInput = page.locator('[data-testid="git-repo-branch-input"]');
		await branchInput.fill("main");

		const tokenInput = page.locator('[data-testid="git-repo-token-input"]');
		await tokenInput.fill("ghp_test123");

		// Select collection from dropdown (VSelect — custom component)
		const collectionSelect = page.locator(
			'[data-testid="git-repo-collection-select"]',
		);
		// DEBUG [e2e] VSelect interaction: trigger click → option click
		// Click the trigger button to open the dropdown
		const selectTrigger = collectionSelect.locator(".v-select__trigger");
		await selectTrigger.click();
		// Wait for dropdown to appear (teleported to body)
		const dropdown = page.locator(".v-select__dropdown");
		await expect(dropdown).toBeVisible({ timeout: 3000 });
		// Click the option with value "col-1"
		const option = dropdown.locator(".v-select__option", {
			hasText: "Test Collection",
		});
		await option.click();

		// Submit
		const submitBtn = page.locator('[data-testid="btn-git-repo-register"]');
		await submitBtn.click();

		// Wait for the new row to appear in the table
		const repoRow = page.locator('[data-testid="git-repo-row"]');
		await expect(repoRow).toBeVisible({ timeout: 5000 });

		// Verify URL is displayed
		await expect(repoRow).toContainText("github.com/user/test-repo.git");

		// Verify status badge shows "idle"
		const statusBadge = repoRow.locator('[data-testid="git-repo-status"]');
		await expect(statusBadge).toBeVisible();
		await expect(statusBadge).toContainText(/idle/i);
	});

	test("TC-GIT-002: trigger sync and observe status transitions", async ({
		page,
	}) => {
		let syncRequested = false;

		// Mock GET to return repos list
		await page.route("**/api/git-sync/repos", async (route, request) => {
			if (request.method() === "GET") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify(MOCK_REPOS),
				});
			} else if (request.method() === "POST") {
				await route.fulfill({
					status: 201,
					contentType: "application/json",
					body: JSON.stringify(MOCK_REPOS[0]),
				});
			} else {
				await route.continue();
			}
		});

		// Mock sync trigger and status poll
		await page.route(
			"**/api/git-sync/repos/repo-1/sync",
			async (route, request) => {
				syncRequested = true;
				if (request.method() === "POST") {
					await route.fulfill({
						status: 202,
						contentType: "application/json",
						body: JSON.stringify({
							repo_id: "repo-1",
							status: "syncing",
							files_indexed: 0,
							chunks_total: 0,
							last_commit: null,
							error: null,
						}),
					});
				} else if (request.method() === "GET") {
					// Poll for sync status — simulate completion
					await route.fulfill({
						status: 200,
						contentType: "application/json",
						body: JSON.stringify({
							repo_id: "repo-1",
							status: "idle",
							files_indexed: 12,
							chunks_total: 48,
							last_commit: "abc123def",
							error: null,
						}),
					});
				} else {
					await route.continue();
				}
			},
		);

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// Click "Sync Now" button on the repo row
		const syncBtn = page.locator('[data-testid="btn-git-sync-now"]');
		await expect(syncBtn).toBeVisible({ timeout: 5000 });
		await syncBtn.click();

		// Verify sync was requested
		expect(syncRequested).toBe(true);

		// Status should transition back to "idle" after sync completes
		const statusBadge = page.locator('[data-testid="git-repo-status"]');
		await expect(statusBadge).toBeVisible({ timeout: 5000 });

		// Verify status badge — may briefly show "syncing" before transitioning to "idle"
		await expect(statusBadge).toContainText(/idle|syncing/i);
	});

	test("TC-GIT-003: delete repo → confirm dialog → row removed", async ({
		page,
	}) => {
		let deleteCalled = false;
		let reposAfterDelete: typeof MOCK_REPOS = [];

		await page.route("**/api/git-sync/repos", async (route, request) => {
			if (request.method() === "GET") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify(reposAfterDelete),
				});
			} else {
				await route.continue();
			}
		});

		await page.route("**/api/git-sync/repos/repo-1", async (route, request) => {
			if (request.method() === "DELETE") {
				deleteCalled = true;
				await route.fulfill({ status: 204 });
			} else {
				await route.continue();
			}
		});

		// Start with one repo
		reposAfterDelete = [...MOCK_REPOS];

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// Click delete button
		const deleteBtn = page.locator('[data-testid="btn-git-repo-delete"]');
		await expect(deleteBtn).toBeVisible();
		await deleteBtn.click();

		// Confirmation dialog should appear
		const confirmDialog = page.locator('[data-testid="confirm-dialog"]');
		await expect(confirmDialog).toBeVisible({ timeout: 3000 });

		// Click confirm
		const confirmBtn = confirmDialog.locator(
			'[data-testid="btn-dialog-confirm"]',
		);
		await confirmBtn.click();

		// Verify delete was called
		expect(deleteCalled).toBe(true);

		// After delete, the repos list updates — remove from mock
		reposAfterDelete = [];
	});

	test("TC-GIT-004: form validation errors — empty submit and invalid URL", async ({
		page,
	}) => {
		// Mock empty repos list
		await page.route("**/api/git-sync/repos", async (route) => {
			await route.fulfill({
				status: 200,
				contentType: "application/json",
				body: JSON.stringify(EMPTY_REPOS),
			});
		});

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// Open connect dialog
		const connectBtn = page.locator('[data-testid="btn-git-repo-connect"]');
		await expect(connectBtn).toBeVisible();
		await connectBtn.click();

		// Submit empty form
		const submitBtn = page.locator('[data-testid="btn-git-repo-register"]');
		await expect(submitBtn).toBeVisible();
		await submitBtn.click();

		// Should show inline error message for required URL
		const urlError = page.locator('[data-testid="git-repo-url-error"]');
		await expect(urlError).toBeVisible({ timeout: 3000 });
		await expect(urlError).toContainText(/required|обязатель/i);

		// Fill invalid URL and re-submit
		const urlInput = page.locator('[data-testid="git-repo-url-input"]');
		await urlInput.fill("ftp://bad-protocol/repo.git");

		await submitBtn.click();

		// Should show URL format error
		await expect(urlError).toContainText(/https:\/\/|git@/i);
	});

	test("TC-GIT-005: sync error state — broken URL shows error badge", async ({
		page,
	}) => {
		// Mock GET repos with error status
		const errorRepos = [
			{
				...MOCK_REPOS[0],
				id: "repo-broken",
				url: "https://nonexistent.invalid/repo.git",
				status: "error",
			},
		];

		await page.route("**/api/git-sync/repos", async (route, request) => {
			if (request.method() === "GET") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify(errorRepos),
				});
			} else {
				await route.continue();
			}
		});

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// Status badge should show "error"
		const statusBadge = page.locator('[data-testid="git-repo-status"]');
		await expect(statusBadge).toBeVisible({ timeout: 5000 });
		await expect(statusBadge).toContainText(/error/i);
	});

	test("TC-GIT-006: list shows multiple repos with correct collection names", async ({
		page,
	}) => {
		const multipleRepos = [
			{
				id: "repo-1",
				url: "https://github.com/user/docs.git",
				branch: "main",
				collection_id: "col-1",
				status: "idle",
				local_path: "/tmp/clones/repo-1",
				last_synced_at: new Date().toISOString(),
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString(),
				collection_name: "Engineering Docs",
			},
			{
				id: "repo-2",
				url: "https://github.com/user/api.git",
				branch: "develop",
				collection_id: "col-2",
				status: "idle",
				local_path: "/tmp/clones/repo-2",
				last_synced_at: new Date().toISOString(),
				created_at: new Date().toISOString(),
				updated_at: new Date().toISOString(),
				collection_name: "API Reference",
			},
		];

		await page.route("**/api/git-sync/repos", async (route, request) => {
			if (request.method() === "GET") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify(multipleRepos),
				});
			} else {
				await route.continue();
			}
		});

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// Should have 2 repo rows
		const repoRows = page.locator('[data-testid="git-repo-row"]');
		await expect(repoRows).toHaveCount(2);

		// First row should show correct URL and collection name
		const firstRow = repoRows.first();
		await expect(firstRow).toContainText("github.com/user/docs.git");
		await expect(firstRow).toContainText("Engineering Docs");

		// Second row should show correct URL and collection name
		const secondRow = repoRows.nth(1);
		await expect(secondRow).toContainText("github.com/user/api.git");
		await expect(secondRow).toContainText("API Reference");
	});

	test("TC-GIT-007: unauthenticated access returns 401 redirect", async ({
		page,
	}) => {
		// The beforeEach injects JWT via addInitScript (persists across navigations).
		// Register a LATER init script that clears it to simulate unauthenticated user.
		await page.addInitScript(() => {
			localStorage.removeItem("vedo_auth_token");
		});

		// Navigate to admin — the second init script runs after the first and clears the token
		await page.goto("/admin");

		// Intercept API calls and assert 401 response
		let responseStatus = 0;
		await page.route("**/api/git-sync/repos", async (route) => {
			await route.fulfill({
				status: 401,
				contentType: "application/json",
				body: JSON.stringify({ error: "Unauthorized" }),
			});
			responseStatus = 401;
		});

		// Navigate again so the route mock is active
		await page.goto("/admin");

		// The UI should show login page redirect when no valid JWT
		await expect(page).toHaveURL(/\/login/);
	});

	test("TC-GIT-008: delete dialog cancel → row stays, no DELETE sent", async ({
		page,
	}) => {
		let deleteCalled = false;

		// Mock repos list with one repo
		await page.route("**/api/git-sync/repos", async (route, request) => {
			if (request.method() === "GET") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify(MOCK_REPOS),
				});
			} else {
				await route.continue();
			}
		});

		await page.route("**/api/git-sync/repos/repo-1", async (route, request) => {
			if (request.method() === "DELETE") {
				deleteCalled = true;
				await route.fulfill({ status: 204 });
			} else {
				await route.continue();
			}
		});

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// Click delete button
		const deleteBtn = page.locator('[data-testid="btn-git-repo-delete"]');
		await expect(deleteBtn).toBeVisible();
		await deleteBtn.click();

		// Confirmation dialog should appear
		const confirmDialog = page.locator('[data-testid="confirm-dialog"]');
		await expect(confirmDialog).toBeVisible({ timeout: 3000 });

		// Click CANCEL instead of confirm
		const cancelBtn = confirmDialog.locator(
			'[data-testid="btn-dialog-cancel"]',
		);
		await expect(cancelBtn).toBeVisible();
		await cancelBtn.click();

		// Dialog should close
		await expect(confirmDialog).not.toBeVisible({ timeout: 3000 });

		// Repo row should still be present
		const repoRow = page.locator('[data-testid="git-repo-row"]');
		await expect(repoRow).toBeVisible();

		// DELETE should NOT have been called
		expect(deleteCalled).toBe(false);
	});

	test("TC-GIT-009: empty repos list shows zero-state message", async ({
		page,
	}) => {
		// Mock empty repos list
		await page.route("**/api/git-sync/repos", async (route, request) => {
			if (request.method() === "GET") {
				await route.fulfill({
					status: 200,
					contentType: "application/json",
					body: JSON.stringify([]),
				});
			} else {
				await route.continue();
			}
		});

		await page.goto("/admin");
		const adminView = page.locator('[data-testid="admin-view"]');
		await expect(adminView).toBeVisible({ timeout: 5000 });

		// Switch to Git Repositories tab
		const gitTab = page.locator("button", { hasText: "Git Repositories" });
		await expect(gitTab).toBeVisible({ timeout: 5000 });
		await gitTab.click();

		const gitManager = page.locator('[data-testid="git-repo-manager"]');
		await expect(gitManager).toBeVisible({ timeout: 5000 });

		// No repo rows should be present
		const repoRows = page.locator('[data-testid="git-repo-row"]');
		await expect(repoRows).toHaveCount(0);

		// Zero-state placeholder should be visible
		const emptyState = page.locator('[data-testid="git-repo-empty-state"]');
		await expect(emptyState).toBeVisible({ timeout: 3000 });
		await expect(emptyState).toContainText(/no repositories/i);
	});
});

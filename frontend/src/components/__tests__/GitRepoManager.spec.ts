import { useCollectionStore } from "@/stores/collections";
import { mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";
import GitRepoManager from "../GitRepoManager.vue";

const apiMock = vi.hoisted(() => ({
	get: vi.fn(),
	getGitRepos: vi.fn(),
	createGitRepo: vi.fn(),
	triggerSync: vi.fn(),
	deleteGitRepo: vi.fn(),
}));

vi.mock("@/api/client", () => ({
	api: apiMock,
}));

describe("GitRepoManager", () => {
	beforeEach(() => {
		document.body.innerHTML = "";
		vi.clearAllMocks();
		apiMock.getGitRepos.mockResolvedValue([]);
	});

	it("connects repositories to the active collection without a collection field", async () => {
		const wrapper = mount(GitRepoManager, {
			global: {
				plugins: [createPinia()],
			},
		});
		const collectionStore = useCollectionStore();
		collectionStore.collections = [
			{
				id: "collection-1",
				name: "Technical Docs",
				created_at: "2026-06-19T00:00:00Z",
				document_count: 2,
			},
		];
		collectionStore.setActiveCollection("collection-1");
		apiMock.createGitRepo.mockResolvedValue({
			id: "repo-1",
			url: "https://github.com/user/docs.git",
			branch: "main",
			collection_id: "collection-1",
			collection_name: "Technical Docs",
			status: "idle",
			local_path: "/tmp/clones/repo-1",
			created_at: "2026-06-19T00:00:00Z",
			updated_at: "2026-06-19T00:00:00Z",
		});
		await nextTick();

		await wrapper.get('[data-testid="btn-git-repo-connect"]').trigger("click");
		await nextTick();

		expect(
			document.body.querySelector('[data-testid="git-repo-collection-select"]'),
		).toBeNull();
		expect(document.body.textContent).toContain("Technical Docs");

		const urlInput = document.body.querySelector<HTMLInputElement>(
			'[data-testid="git-repo-url-input"]',
		);
		if (!urlInput) {
			throw new Error("Expected repository URL input to be rendered.");
		}
		urlInput.value = "https://github.com/user/docs.git";
		urlInput.dispatchEvent(new Event("input", { bubbles: true }));
		await nextTick();

		document.body
			.querySelector<HTMLElement>('[data-testid="btn-git-repo-register"]')
			?.click();

		expect(apiMock.createGitRepo).toHaveBeenCalledWith({
			url: "https://github.com/user/docs.git",
			branch: "main",
			access_token: undefined,
			collection_id: "collection-1",
		});
	});
});

import ChatView from "@/views/ChatView.vue";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { nextTick } from "vue";

const apiMock = vi.hoisted(() => ({
	get: vi.fn((path: string) => {
		if (path === "/collections") {
			return Promise.resolve([
				{
					id: "collection-1",
					name: "Technical Docs",
					created_at: "2026-06-19T00:00:00Z",
					document_count: 2,
				},
			]);
		}
		if (path === "/sessions") {
			return Promise.resolve([]);
		}
		return Promise.resolve([]);
	}),
}));

vi.mock("@/api/client", () => ({
	api: apiMock,
	ApiError: class ApiError extends Error {
		constructor(
			public status: number,
			message: string,
		) {
			super(message);
		}
	},
	getAccessToken: vi.fn(() => null),
}));

describe("ChatWindow (ChatView)", () => {
	beforeEach(() => {
		document.body.innerHTML = "";
		vi.clearAllMocks();
		setActivePinia(createPinia());
	});

	it("renders welcome screen when no messages", () => {
		const wrapper = mount(ChatView);
		expect(wrapper.find('[data-testid="welcome-message"]').exists()).toBe(true);
	});

	it("has a send button", () => {
		const wrapper = mount(ChatView);
		expect(wrapper.find('[data-testid="btn-send"]').exists()).toBe(true);
	});

	it("has input textarea", () => {
		const wrapper = mount(ChatView);
		expect(wrapper.find('[data-testid="chat-input"]').exists()).toBe(true);
	});

	it("does not show cancel button when not loading", () => {
		const wrapper = mount(ChatView);
		expect(wrapper.find('[data-testid="btn-cancel"]').exists()).toBe(false);
	});

	it("shows cancel button when loading", async () => {
		const wrapper = mount(ChatView);
		const { useChatStore } = await import("@/stores/chat");
		const chatStore = useChatStore();
		chatStore.isLoading = true;
		await wrapper.vm.$nextTick();
		expect(wrapper.find('[data-testid="btn-cancel"]').exists()).toBe(true);
	});

	it("shows collection options when the chat collection selector is opened", async () => {
		mount(ChatView, {
			attachTo: document.body,
		});

		await nextTick();
		await new Promise((resolve) => setTimeout(resolve, 0));
		await nextTick();

		const trigger = document.body.querySelector<HTMLElement>(
			'[data-testid="collection-select"] .v-select__trigger',
		);
		if (!trigger) {
			throw new Error(
				"Expected chat collection selector trigger to be rendered.",
			);
		}

		trigger.click();
		await nextTick();

		const dropdown = document.body.querySelector<HTMLElement>(
			'[data-testid="collection-select-dropdown"]',
		);
		expect(dropdown).not.toBeNull();
		expect(dropdown?.textContent).toContain("Technical Docs");
	});
});

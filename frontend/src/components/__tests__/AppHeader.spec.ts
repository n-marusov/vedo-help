import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";
import { h } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import AppHeader from "../AppHeader.vue";

// Stub router component for <router-link> support
const router = createRouter({
	history: createWebHistory(),
	routes: [
		{ path: "/", name: "chat", component: { render: () => h("div", "Chat") } },
	],
});

describe("AppHeader", () => {
	it("renders VEDO branding and the theme toggle", async () => {
		router.push("/");
		await router.isReady();
		const wrapper = mount(AppHeader, {
			global: {
				plugins: [router],
			},
		});

		expect(wrapper.get('[data-testid="app-header"]').text()).toContain("VEDO");
		expect(wrapper.find('[data-testid="theme-toggle"]').exists()).toBe(true);
	});
});

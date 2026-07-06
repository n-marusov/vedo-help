import SettingsPanel from '@/components/SettingsPanel.vue';
import { flushPromises, mount } from '@vue/test-utils';
import { createPinia, setActivePinia } from 'pinia';
import { describe, expect, it, vi } from 'vitest';
import { nextTick } from 'vue';

const { mockGetSettings, mockUpdateSettings } = vi.hoisted(() => ({
  mockGetSettings: vi.fn(),
  mockUpdateSettings: vi.fn(),
}));

vi.mock('@/api/client', () => ({
  api: {
    getSettings: mockGetSettings,
    updateSettings: mockUpdateSettings,
  },
}));

function mountWithPinia() {
  const pinia = createPinia();
  setActivePinia(pinia);
  return mount(SettingsPanel, {
    global: {
      plugins: [pinia],
    },
  });
}

const DEFAULT_API_RESPONSE = {
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
};

describe('SettingsPanel', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockGetSettings.mockResolvedValue({ ...DEFAULT_API_RESPONSE });
    mockUpdateSettings.mockResolvedValue({ ...DEFAULT_API_RESPONSE });
  });

  it('shows skeleton loader while loading', () => {
    // Never resolve so it stays in loading state
    mockGetSettings.mockImplementation(() => new Promise(() => {}));
    const wrapper = mountWithPinia();
    expect(wrapper.find('[data-testid="settings-panel"]').exists()).toBe(true);
    // VSkeleton renders elements with data-testid="skeleton"
    expect(wrapper.find('[data-testid="skeleton"]').exists()).toBe(true);
  });

  it('loads settings and populates form on mount', async () => {
    const wrapper = mountWithPinia();
    // Wait for async loadSettings to complete
    await flushPromises();

    expect(mockGetSettings).toHaveBeenCalledTimes(1);

    // After loading, skeleton should be gone and form visible
    expect(wrapper.find('[data-testid="skeleton"]').exists()).toBe(false);

    // Model section should be visible
    expect(wrapper.text()).toContain('Models');
    expect(wrapper.text()).toContain('LLM Model');
    expect(wrapper.text()).toContain('Embedding Model');

    // Default model values should appear
    expect(wrapper.text()).toContain('Claude Sonnet 4.6');
    expect(wrapper.text()).toContain('all-MiniLM-L6-v2');
  });

  it('shows Save button disabled when nothing changed', async () => {
    const wrapper = mountWithPinia();
    await flushPromises();

    // Find save button by its text
    const saveBtn = wrapper.findAll('button').filter((b) => b.text().includes('Save Changes'))[0];
    expect(saveBtn).toBeDefined();
    // Save button should be disabled (no changes yet)
    expect(saveBtn.attributes('disabled')).toBeDefined();
  });

  it('enables Save button after changing LLM model', async () => {
    const wrapper = mountWithPinia();
    await flushPromises();

    // Change a setting via component internal data
    const vm = wrapper.vm as unknown as {
      form: Record<string, unknown>;
      changed: boolean;
    };
    vm.form.llm_model = 'openai/gpt-5.5';
    await nextTick();

    expect(vm.changed).toBe(true);

    // Save button should now be enabled (no disabled attribute)
    const saveBtn = wrapper.findAll('button').filter((b) => b.text().includes('Save Changes'))[0];
    expect(saveBtn).toBeDefined();
    expect(saveBtn.attributes('disabled')).toBeUndefined();
  });

  it('saves settings and shows success toast', async () => {
    const wrapper = mountWithPinia();
    await flushPromises();

    // Modify a field via component internals
    const vm = wrapper.vm as unknown as {
      form: Record<string, unknown>;
      saveSettings: () => Promise<void>;
    };
    vm.form.llm_model = 'openai/gpt-5.5';
    vm.form.embedding_model = 'openai/text-embedding-3-small';
    await nextTick();

    // Call save directly
    await vm.saveSettings();

    expect(mockUpdateSettings).toHaveBeenCalledTimes(1);
    expect(mockUpdateSettings).toHaveBeenCalledWith(
      expect.objectContaining({
        llm_model: 'openai/gpt-5.5',
        embedding_model: 'openai/text-embedding-3-small',
      }),
    );

    // Success toast should be shown
    expect(wrapper.text()).toContain('Settings saved successfully');
  });

  it('shows error toast when save fails', async () => {
    mockUpdateSettings.mockRejectedValue(new Error('Network error'));

    const wrapper = mountWithPinia();
    await flushPromises();

    const vm = wrapper.vm as unknown as {
      form: Record<string, unknown>;
      saveSettings: () => Promise<void>;
    };
    vm.form.llm_model = 'openai/gpt-5.5';
    vm.form.embedding_model = 'openai/text-embedding-3-small';
    await nextTick();

    await vm.saveSettings();

    expect(mockUpdateSettings).toHaveBeenCalledTimes(1);
    expect(wrapper.text()).toContain('Failed to save settings');
    expect(wrapper.text()).toContain('Network error');
  });

  it('shows error toast when load fails', async () => {
    mockGetSettings.mockRejectedValue(new Error('Server unavailable'));

    const wrapper = mountWithPinia();
    await flushPromises();

    expect(wrapper.text()).toContain('Failed to load settings');
    expect(wrapper.text()).toContain('Server unavailable');
  });

  it('opens reset dialog and resets to defaults', async () => {
    const wrapper = mountWithPinia();
    await flushPromises();

    const vm = wrapper.vm as unknown as {
      form: Record<string, unknown>;
      showResetDialog: boolean;
      resetToDefaults: () => void;
    };

    // First change something
    vm.form.llm_model = 'openai/gpt-5.5';
    await nextTick();
    expect(vm.form.llm_model).toBe('openai/gpt-5.5');

    // Open reset dialog
    vm.showResetDialog = true;
    await nextTick();

    // VDialog uses Teleport — content is outside wrapper
    // Verify via reactive state instead
    expect(vm.showResetDialog).toBe(true);

    // Reset
    vm.resetToDefaults();
    await nextTick();

    // Values should be back to defaults
    expect(vm.form.llm_model).toBe('anthropic/claude-sonnet-4.6');
    expect(vm.form.embedding_model).toBe('sentence-transformers/all-minilm-l6-v2');
    expect(vm.showResetDialog).toBe(false);

    // Toast about reset should show
    expect(wrapper.text()).toContain('reset to defaults');
  });

  it('renders all model selectors in Models section', async () => {
    const wrapper = mountWithPinia();
    await flushPromises();

    const html = wrapper.html();

    // Both VSelect trigger buttons should show default labels
    expect(html).toContain('Claude Sonnet 4.6');
    expect(html).toContain('all-MiniLM-L6-v2');

    // Rerank model input should exist
    expect(html).toContain('anthropic/claude-sonnet-4.6');

    // Models section should be present
    expect(wrapper.text()).toContain('Main output model');
    expect(wrapper.text()).toContain('Model used for generating');
    expect(wrapper.text()).toContain('model used for reranking');
  });

  it('shows Saving... text while saving', async () => {
    // Don't resolve the save call to keep it in pending state
    mockUpdateSettings.mockImplementation(() => new Promise(() => {}));

    const wrapper = mountWithPinia();
    await flushPromises();

    const vm = wrapper.vm as unknown as {
      form: Record<string, unknown>;
      saveSettings: () => Promise<void>;
      saving: boolean;
    };
    vm.form.llm_model = 'openai/gpt-5.5';
    await nextTick();

    // Start saving
    vm.saveSettings();
    await nextTick();

    expect(vm.saving).toBe(true);
    expect(wrapper.text()).toContain('Saving...');
  });
});

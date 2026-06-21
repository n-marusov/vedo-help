import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';
import { defineComponent, h } from 'vue';

// RED phase: VSkeleton.vue does not exist yet (T9 — Pencil design-first).
// Use a stub that compiles but renders the target shape to provide a valid
// import for vitest. The `.skip` ensures these tests never run; they document
// the behaviour that T9 must satisfy.
const VSkeleton = defineComponent({
  name: 'VSkeleton',
  props: {
    variant: { type: String, default: 'text' },
    rows: { type: Number, default: 1 },
  },
  setup() {
    return () => h('div', { 'data-testid': 'skeleton' }, '[[PLACEHOLDER — T9]]');
  },
});

describe('VSkeleton', () => {
  it.skip('renders with data-testid="skeleton"', () => {
    const wrapper = mount(VSkeleton);
    expect(wrapper.find('[data-testid="skeleton"]').exists()).toBe(true);
  });

  it.skip('variant="circle" adds class skeleton-circle', () => {
    const wrapper = mount(VSkeleton, { props: { variant: 'circle' } });
    expect(wrapper.classes()).toContain('skeleton-circle');
  });

  it.skip('rows=4 renders 4 skeleton-line children', () => {
    const wrapper = mount(VSkeleton, { props: { rows: 4 } });
    expect(wrapper.findAll('.skeleton-line')).toHaveLength(4);
  });
});

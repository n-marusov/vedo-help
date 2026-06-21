import VSkeleton from '@/components/ui/VSkeleton.vue';
import { mount } from '@vue/test-utils';
import { describe, expect, it } from 'vitest';

describe('VSkeleton', () => {
  it('renders with data-testid="skeleton"', () => {
    const wrapper = mount(VSkeleton);
    expect(wrapper.find('[data-testid="skeleton"]').exists()).toBe(true);
  });

  it('variant="circle" adds class skeleton-circle', () => {
    const wrapper = mount(VSkeleton, { props: { variant: 'circle' } });
    expect(wrapper.classes()).toContain('skeleton-circle');
  });

  it('rows=4 renders 4 skeleton-line children', () => {
    const wrapper = mount(VSkeleton, { props: { rows: 4 } });
    expect(wrapper.findAll('.skeleton-line')).toHaveLength(4);
  });
});

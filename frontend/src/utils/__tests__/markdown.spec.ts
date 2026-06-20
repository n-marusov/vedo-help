import { describe, expect, it } from 'vitest';
import { decodeCode, encodeCode, renderMarkdown } from '../markdown';

describe('renderMarkdown', () => {
  it('renders bold text', () => {
    const result = renderMarkdown('**bold**');
    expect(result).toContain('<strong>bold</strong>');
  });

  it('renders code block with hljs classes', () => {
    const result = renderMarkdown('```python\nprint("hello")\n```');
    expect(result).toContain('hljs');
    expect(result).toContain('code-block-wrapper');
    expect(result).toContain('copy-code-btn');
    expect(result).toContain('code-lang-label');
  });

  it('returns empty string for empty input', () => {
    expect(renderMarkdown('')).toBe('');
  });

  it('renders GFM table', () => {
    const result = renderMarkdown('| A | B |\n|---|---|\n| 1 | 2 |');
    expect(result).toContain('<table>');
    expect(result).toContain('<th>A</th>');
    expect(result).toContain('<td>1</td>');
  });

  it('renders blockquote', () => {
    const result = renderMarkdown('> quote');
    expect(result).toContain('<blockquote>');
  });

  it('renders unordered list', () => {
    const result = renderMarkdown('- item');
    expect(result).toContain('<ul>');
    expect(result).toContain('<li>item</li>');
  });

  it('handles invalid input gracefully', () => {
    const result = renderMarkdown(null as unknown as string);
    expect(result).toBe('');
  });
});

describe('encodeCode / decodeCode', () => {
  it('roundtrips code content', () => {
    const code = 'print("hello")\nconst x = 1;';
    expect(decodeCode(encodeCode(code))).toBe(code);
  });

  it('handles special characters', () => {
    const code = '<script>alert("xss")</script>';
    expect(decodeCode(encodeCode(code))).toBe(code);
  });
});

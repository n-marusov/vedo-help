import hljs from 'highlight.js/lib/core';
import bash from 'highlight.js/lib/languages/bash';
import css from 'highlight.js/lib/languages/css';
import javascript from 'highlight.js/lib/languages/javascript';
import json from 'highlight.js/lib/languages/json';
import markdown from 'highlight.js/lib/languages/markdown';
import plaintext from 'highlight.js/lib/languages/plaintext';
import python from 'highlight.js/lib/languages/python';
import rust from 'highlight.js/lib/languages/rust';
import sql from 'highlight.js/lib/languages/sql';
import typescript from 'highlight.js/lib/languages/typescript';
import xml from 'highlight.js/lib/languages/xml';

// Register common languages
hljs.registerLanguage('javascript', javascript);
hljs.registerLanguage('typescript', typescript);
hljs.registerLanguage('python', python);
hljs.registerLanguage('rust', rust);
hljs.registerLanguage('bash', bash);
hljs.registerLanguage('sh', bash);
hljs.registerLanguage('shell', bash);
hljs.registerLanguage('json', json);
hljs.registerLanguage('xml', xml);
hljs.registerLanguage('html', xml);
hljs.registerLanguage('css', css);
hljs.registerLanguage('sql', sql);
hljs.registerLanguage('markdown', markdown);
hljs.registerLanguage('text', plaintext);
hljs.registerLanguage('plaintext', plaintext);
import { Marked, Renderer } from 'marked';

/**
 * Escape HTML special characters for safe insertion into attributes.
 */
function escapeHtml(str: string): string {
  return str
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#39;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');
}

/**
 * Encode code text for safe storage in a data attribute.
 * Uses encodeURIComponent which produces only ASCII chars safe in HTML attributes.
 */
export function encodeCode(code: string): string {
  return encodeURIComponent(code).replace(/'/g, '%27');
}

/**
 * Decode code text from a data attribute.
 */
export function decodeCode(encoded: string): string {
  return decodeURIComponent(encoded);
}

const renderer = new Renderer();

renderer.code = (code: string, language?: string): string => {
  const lang = language || '';
  let highlighted: string;

  if (lang && hljs.getLanguage(lang)) {
    try {
      highlighted = hljs.highlight(code, {
        language: lang,
        ignoreIllegals: true,
      }).value;
      console.debug('[markdown] highlighted code block', {
        language: lang || 'auto',
        length: code.length,
      });
    } catch {
      console.warn('[markdown] highlight failed', {
        language: lang,
        length: code.length,
      });
      try {
        highlighted = hljs.highlightAuto(code).value;
        console.debug('[markdown] highlighted code block', {
          language: lang || 'auto',
          length: code.length,
        });
      } catch {
        console.warn('[markdown] highlight failed', {
          language: lang,
          length: code.length,
        });
        highlighted = escapeHtml(code);
      }
    }
  } else {
    try {
      highlighted = hljs.highlightAuto(code).value;
      console.debug('[markdown] highlighted code block', {
        language: lang || 'auto',
        length: code.length,
      });
    } catch {
      console.warn('[markdown] highlight failed', {
        language: lang,
        length: code.length,
      });
      highlighted = escapeHtml(code);
    }
  }

  const encoded = encodeCode(code);
  const langLabel = lang ? escapeHtml(lang) : 'Code';
  return `<div class="code-block-wrapper">
<div class="code-block-header">
<span class="code-lang-label">${langLabel}</span>
<button class="copy-code-btn" data-code="${encoded}" type="button">Copy</button>
</div>
<pre><code class="hljs${lang ? ` language-${escapeHtml(lang)}` : ''}">${highlighted}</code></pre>
</div>`;
};

const marked = new Marked({
  renderer,
  breaks: false,
  gfm: true,
});

/**
 * Parse markdown content to HTML with syntax highlighting and copy buttons.
 */
export function renderMarkdown(content: string): string {
  if (!content) return '';
  try {
    console.debug('[markdown] parse start', { length: content.length });
    return (marked.parse(content, { async: false }) as string) || '';
  } catch (err) {
    console.warn('[renderMarkdown] marked.parse failed', err);
    return content;
  }
}

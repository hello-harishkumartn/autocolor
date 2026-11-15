import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, readdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';

// Harish's style for user-facing copy: no Oxford comma, no comma before "and" or "or", no em or en dashes.
const root = new URL('../../', import.meta.url).pathname;
const docs = ['README.md', 'RESULT.md', 'GLOSSARY.md', 'tests/fixtures/LICENSES.md', ...readdirSync(join(root, 'docs')).filter((f) => f.endsWith('.md')).map((f) => `docs/${f}`)];
const files = ['app/index.html', 'app/app.js', 'app/names.js', 'app/worker.js', ...docs].filter((f) => existsSync(join(root, f)));

const visible = (file, text) => {
  if (file.endsWith('.html')) return text.replace(/<style[\s\S]*?<\/style>|<script[\s\S]*?<\/script>/g, '').replace(/<[^>]+>/g, ' ');
  if (file.endsWith('.js')) return [...text.matchAll(/(['`])((?:(?!\1).)*)\1/g)].map((m) => m[2]).join('\n'); // string literals only
  return text.replace(/```[\s\S]*?```/g, '').replace(/`[^`]*`/g, 'code'); // markdown without code
};

test('user-facing copy follows the house style', () => {
  const problems = [];
  for (const f of files) {
    const text = visible(f, readFileSync(join(root, f), 'utf8'));
    text.split('\n').forEach((line, i) => {
      for (const m of line.matchAll(/,\s+(and|or)\b|[–—]/g)) problems.push(`${f}:${i + 1}: "${line.trim().slice(Math.max(0, m.index - 30), m.index + 15)}"`);
    });
  }
  assert.deepEqual(problems, []);
});

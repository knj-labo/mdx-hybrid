import { describe, it, expect, beforeAll, afterAll } from 'bun:test';
import { ShikiWorkerPool, createPooledHighlighter } from './shiki-pool.js';

describe('ShikiWorkerPool', () => {
  let pool;

  beforeAll(async () => {
    pool = new ShikiWorkerPool(2); // Use 2 workers for testing
    await pool.ready();
  });

  afterAll(async () => {
    if (pool) {
      await pool.terminate();
    }
  });

  it('initializes with the correct number of workers', () => {
    expect(pool.size).toBe(2);
  });

  it('highlights simple code', async () => {
    const html = await pool.highlight('const x = 1;', 'javascript');

    expect(html).toContain('<pre class="astro-code');
    expect(html).toContain('const');
    expect(html).toContain('x');
  });

  it('highlights code without language', async () => {
    const html = await pool.highlight('hello world', null);

    expect(html).toContain('<pre class="astro-code');
    expect(html).toContain('hello');
  });

  it('processes multiple highlights in parallel', async () => {
    const codes = [
      { code: 'const a = 1;', lang: 'javascript' },
      { code: 'def foo(): pass', lang: 'python' },
      { code: '<div>hello</div>', lang: 'html' },
      { code: 'SELECT * FROM users', lang: 'sql' },
    ];

    const results = await Promise.all(
      codes.map(({ code, lang }) => pool.highlight(code, lang))
    );

    expect(results).toHaveLength(4);
    results.forEach((html) => {
      expect(html).toContain('<pre class="astro-code');
    });
  });

  it('handles many concurrent requests with queue', async () => {
    const count = 10;
    const promises = [];

    for (let i = 0; i < count; i++) {
      promises.push(pool.highlight(`const x${i} = ${i};`, 'javascript'));
    }

    const results = await Promise.all(promises);

    expect(results).toHaveLength(count);
    results.forEach((html, i) => {
      expect(html).toContain('<pre class="astro-code');
      expect(html).toContain(`x${i}`);
    });
  });
});

describe('createPooledHighlighter', () => {
  let pool;
  let highlighter;

  beforeAll(async () => {
    pool = new ShikiWorkerPool(1);
    await pool.ready();
    highlighter = createPooledHighlighter(pool);
  });

  afterAll(async () => {
    if (pool) {
      await pool.terminate();
    }
  });

  it('returns a function that highlights code', async () => {
    const html = await highlighter('const x = 1;', 'javascript');

    expect(html).toContain('<pre class="astro-code');
    expect(html).toContain('const');
  });
});

describe('ShikiWorkerPool termination', () => {
  it('throws error after termination', async () => {
    const tempPool = new ShikiWorkerPool(1);
    await tempPool.ready();
    await tempPool.terminate();

    await expect(tempPool.highlight('code', 'javascript')).rejects.toThrow(
      'Worker pool has been terminated'
    );
  });

  it('can be terminated multiple times safely', async () => {
    const tempPool = new ShikiWorkerPool(1);
    await tempPool.ready();
    await tempPool.terminate();
    await tempPool.terminate(); // Should not throw
  });
});

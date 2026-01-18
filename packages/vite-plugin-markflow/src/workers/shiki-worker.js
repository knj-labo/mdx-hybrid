/**
 * Shiki Worker Thread
 *
 * This worker handles Shiki syntax highlighting in a separate thread,
 * enabling true parallel CPU-bound processing across multiple cores.
 */
import { parentPort, workerData } from 'node:worker_threads';
import { codeToHtml, createCssVariablesTheme } from 'shiki';

// Create the theme once per worker
const theme = createCssVariablesTheme({
  name: 'astro-code',
  variablePrefix: '--astro-code-',
});

// Process highlight requests
parentPort.on('message', async ({ id, code, lang }) => {
  try {
    const html = await codeToHtml(code, {
      lang: lang || 'text',
      theme,
    });

    // Normalize CSS classes to match Astro's expected format
    const normalized = html.replace(/<pre class="([^"]*)"/, (match, classes) => {
      const filtered = classes
        .split(/\s+/)
        .filter((value) => value && value !== 'shiki')
        .join(' ');
      const next = filtered ? `astro-code ${filtered}` : 'astro-code';
      return `<pre class="${next}"`;
    });

    parentPort.postMessage({ id, html: normalized, error: null });
  } catch (error) {
    parentPort.postMessage({ id, html: null, error: error.message });
  }
});

// Signal ready
parentPort.postMessage({ type: 'ready' });

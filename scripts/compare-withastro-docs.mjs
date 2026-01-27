#!/usr/bin/env node
import { promises as fs } from 'node:fs';
import { dirname, resolve, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawn } from 'node:child_process';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const withastroRoot = resolve(
  repoRoot,
  'fixtures/integration/withastro-docs/repo',
);
const defaultSummary = resolve(
  repoRoot,
  'fixtures/integration/withastro-docs/harness-summary.json',
);
const defaultRoutesFile = resolve(
  repoRoot,
  'fixtures/integration/withastro-docs/semantic-routes.txt',
);

function parseArgs(argv) {
  const args = {
    summary: defaultSummary,
    skipInstall: false,
    mode: 'semantic',
    routesFile: null,
    all: false,
  };
  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--summary' && argv[i + 1]) {
      args.summary = resolve(repoRoot, argv[i + 1]);
      i++;
    } else if (arg === '--routes' && argv[i + 1]) {
      args.routesFile = resolve(repoRoot, argv[i + 1]);
      i++;
    } else if (arg === '--all') {
      args.all = true;
    } else if (arg === '--skip-install') {
      args.skipInstall = true;
    } else if (arg === '--mode' && argv[i + 1]) {
      args.mode = argv[i + 1];
      i++;
    } else if (arg.startsWith('--mode=')) {
      args.mode = arg.split('=')[1];
    }
  }
  return args;
}

async function main() {
  const { summary, skipInstall, mode, routesFile, all } = parseArgs(process.argv);
  const distBaseline = resolve(withastroRoot, 'dist-baseline');
  const distMarkflow = resolve(withastroRoot, 'dist-markflow');

  await Promise.all([
    fs.rm(distBaseline, { recursive: true, force: true }),
    fs.rm(distMarkflow, { recursive: true, force: true }),
  ]);

  await cleanBuildArtifacts();
  const baseline = await runBuild(true, { skipInstall });
  await moveDist('baseline', distBaseline);

  await cleanBuildArtifacts();
  const markflow = await runBuild(false, { skipInstall });
  await moveDist('markflow', distMarkflow);

  let semantic = null;
  if (mode === 'semantic') {
    const routeList = await resolveRoutes(routesFile, all);
    semantic = await compareSemantic(distBaseline, distMarkflow, routeList);
  }

  const report = {
    timestamp: new Date().toISOString(),
    baselineMs: Math.round(baseline.durationMs),
    markflowMs: Math.round(markflow.durationMs),
    mode,
    semantic,
    note:
      mode === 'semantic'
        ? 'semantic diff: pass if differences=0'
        : 'build-only; no HTML diff',
  };

  await fs.mkdir(dirname(summary), { recursive: true });
  await fs.writeFile(summary, JSON.stringify(report, null, 2));
  console.log(`Harness summary written to ${summary}`);
  console.log(report);

  if (semantic && semantic.differences > 0) {
    process.exitCode = 1;
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((err) => {
    console.error(err.message || err);
    process.exit(1);
  });
}

async function runBuild(isBaseline, { skipInstall }) {
  const env = { ...process.env, MARKFLOW_HARNESS_OFFLINE: '1' };
  if (!Object.prototype.hasOwnProperty.call(env, 'MARKFLOW_HARNESS_SKIP_FRONTMATTER')) {
    env.MARKFLOW_HARNESS_SKIP_FRONTMATTER = '1';
  }
  if (isBaseline) {
    env.MARKFLOW_HARNESS_BASELINE = '1';
  } else {
    delete env.MARKFLOW_HARNESS_BASELINE;
  }

  const start = process.hrtime.bigint();

  if (!skipInstall) {
    await exec('pnpm', ['--dir', withastroRoot, 'install', '--frozen-lockfile'], env);
  }

  await exec('pnpm', ['--dir', withastroRoot, 'build'], env);

  const end = process.hrtime.bigint();
  return { durationMs: Number(end - start) / 1_000_000 };
}

function exec(cmd, args, env) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(cmd, args, {
      cwd: repoRoot,
      env,
      stdio: 'inherit',
    });
    child.on('close', (code) => {
      if (code === 0) return resolvePromise();
      reject(new Error(`${cmd} ${args.join(' ')} exited with code ${code}`));
    });
  });
}

async function cleanBuildArtifacts() {
  const targets = [
    resolve(withastroRoot, 'dist'),
    resolve(withastroRoot, '.astro'),
    resolve(withastroRoot, '.vite-cache'),
    resolve(withastroRoot, 'node_modules', '.astro'),
    resolve(withastroRoot, 'node_modules', '.vite'),
  ];
  await Promise.all(targets.map((dir) => fs.rm(dir, { recursive: true, force: true })));
}

async function moveDist(label, targetDir) {
  const dist = resolve(withastroRoot, 'dist');
  const exists = await fs
    .stat(dist)
    .then(() => true)
    .catch(() => false);
  if (!exists) {
    throw new Error(`dist not found after ${label} build at ${dist}`);
  }
  await fs.rm(targetDir, { recursive: true, force: true });
  await fs.rename(dist, targetDir);
}

async function compareSemantic(dirA, dirB, routes) {
  if (!routes || routes.length === 0) {
    const filesA = await collectHtml(dirA);
    const filesB = await collectHtml(dirB);
    return compareFiles(filesA, filesB);
  }

  const routesSet = new Set(routes.map(normalizeRoute).filter(Boolean));
  const [mapA, mapB] = await Promise.all([
    collectHtmlByRoute(dirA),
    collectHtmlByRoute(dirB),
  ]);

  const diffs = [];
  let compared = 0;
  let skipped = 0;
  for (const route of routesSet) {
    const a = mapA.get(route);
    const b = mapB.get(route);
    if (a === undefined && b === undefined) {
      skipped += 1;
      continue;
    }
    if (a === undefined || b === undefined) {
      diffs.push({ file: route, reason: 'missing-in-one-side' });
      compared += 1;
      continue;
    }
    const na = normalizeHtml(a);
    const nb = normalizeHtml(b);
    if (na !== nb) {
      const sa = stripTags(na);
      const sb = stripTags(nb);
      if (sa !== sb) {
        diffs.push({
          file: route,
          reason: 'content-diff',
          ...summarizeDiff(sa, sb),
        });
      } else {
        // Structure differs but text content is same (e.g., missing <aside>)
        diffs.push({
          file: route,
          reason: 'structure-diff',
          ...summarizeDiff(na, nb),
        });
      }
    }
    compared += 1;
  }

  if (compared === 0) {
    console.warn(
      '[compare-withastro-docs] No routes matched the dist outputs; check output mode or route list.',
    );
  }

  return {
    compared,
    skipped,
    differences: diffs.length,
    samples: diffs.slice(0, 1),
  };
}

function compareFiles(filesA, filesB) {
  const all = new Set([...filesA.keys(), ...filesB.keys()]);
  const diffs = [];
  for (const rel of all) {
    const a = filesA.get(rel);
    const b = filesB.get(rel);
    if (a === undefined || b === undefined) {
      diffs.push({ file: rel, reason: 'missing-in-one-side' });
      continue;
    }
    const na = normalizeHtml(a);
    const nb = normalizeHtml(b);
    if (na !== nb) {
      const sa = stripTags(na);
      const sb = stripTags(nb);
      if (sa !== sb) {
        diffs.push({
          file: rel,
          reason: 'content-diff',
          ...summarizeDiff(sa, sb),
        });
      } else {
        // Structure differs but text content is same (e.g., missing <aside>)
        diffs.push({
          file: rel,
          reason: 'structure-diff',
          ...summarizeDiff(na, nb),
        });
      }
    }
  }
  return {
    compared: all.size,
    differences: diffs.length,
    samples: diffs.slice(0, 1),
  };
}

async function collectHtml(root) {
  const out = new Map();
  async function walk(dir) {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const full = resolve(dir, entry.name);
      if (entry.isDirectory()) {
        await walk(full);
      } else if (entry.name.endsWith('.html')) {
        const rel = relative(root, full);
        const data = await fs.readFile(full, 'utf8');
        out.set(rel, data);
      }
    }
  }
  await walk(root);
  return out;
}

async function collectHtmlByRoute(root) {
  const out = new Map();
  await walkHtml(root, async (rel, data) => {
    const route = routeFromHtml(rel);
    if (route) {
      out.set(route, data);
    }
  });
  return out;
}

async function walkHtml(root, onFile) {
  async function walk(dir) {
    const entries = await fs.readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const full = resolve(dir, entry.name);
      if (entry.isDirectory()) {
        await walk(full);
      } else if (entry.name.endsWith('.html')) {
        const rel = relative(root, full);
        const data = await fs.readFile(full, 'utf8');
        await onFile(rel, data);
      }
    }
  }
  await walk(root);
}

function routeFromHtml(rel) {
  const normalized = rel.split('\\').join('/');
  if (normalized === 'index.html') return '/';
  if (normalized.endsWith('/index.html')) {
    const base = normalized.slice(0, -'/index.html'.length);
    return normalizeRoute(`/${base}/`);
  }
  if (normalized.endsWith('.html')) {
    return normalizeRoute(`/${normalized.slice(0, -'.html'.length)}`);
  }
  return null;
}

function normalizeRoute(route) {
  if (!route) return null;
  let normalized = route.trim();
  if (!normalized.startsWith('/')) normalized = `/${normalized}`;
  if (normalized !== '/' && !normalized.endsWith('/')) {
    normalized = `${normalized}/`;
  }
  return normalized;
}

async function resolveRoutes(routesFile, all) {
  if (all) return null;
  const file = routesFile || defaultRoutesFile;
  const exists = await fs
    .stat(file)
    .then(() => true)
    .catch(() => false);
  if (!exists) return null;
  return loadRoutes(file);
}

async function loadRoutes(filePath) {
  const data = await fs.readFile(filePath, 'utf8');
  if (filePath.endsWith('.json')) {
    const parsed = JSON.parse(data);
    if (!Array.isArray(parsed)) {
      throw new Error('Routes JSON must be an array.');
    }
    return parsed.map(normalizeRoute).filter(Boolean);
  }
  return data
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line && !line.startsWith('#'))
    .map(normalizeRoute)
    .filter(Boolean);
}

export function normalizeHtml(html) {
  let processed = html;
  const frontmatterRe = /<pre\s+class="frontmatter"[^>]*>[\s\S]*?<\/pre>/gi;
  while (frontmatterRe.test(processed)) {
    processed = processed.replace(frontmatterRe, '');
  }
  // Strip course recommendation boxes (contain randomly selected course content)
  // These are <aside aria-labelledby="learn-astro-course-*"> elements
  // Must be done before stripping generic aside tags
  const courseBoxRe =
    /<aside\b[^>]*aria-labelledby="learn-astro-course[^"]*"[^>]*>[\s\S]*?<\/aside>/gi;
  while (courseBoxRe.test(processed)) {
    processed = processed.replace(courseBoxRe, '');
  }
  // Strip course tracking scripts (querySelectorAll for data-scrimba-cta, data-learn-astro-cta, etc.)
  // These scripts track clicks on course CTA links
  const courseTrackingRe =
    /<script\b[^>]*>document\.querySelectorAll\("a\[data-(?:scrimba|learn-astro)-cta\]"\)[^<]*<\/script>/gi;
  processed = processed.replace(courseTrackingRe, '');
  // Strip math-inline span wrappers (keep content, just remove the wrapping tags)
  // Match entire <span class="math-inline">...</span> and replace with inner content
  const mathInlineRe = /<span\b[^>]*class="[^"]*\bmath-inline\b[^"]*"[^>]*>([\s\S]*?)<\/span>/gi;
  processed = processed.replace(mathInlineRe, '$1');
  processed = processed
    .replace(/<pre><code\b[^>]*>/gi, '<code>')
    .replace(/<\/code><\/pre>/gi, '</code>');

  const noComments = processed.replace(/<!--[\s\S]*?-->/g, '');
  const tagRe = /<[^>]+>/g;
  let out = '';
  let last = 0;
  let m;
  let codeDepth = 0;
  while ((m = tagRe.exec(noComments))) {
    let text = noComments.slice(last, m.index);
    out += codeDepth === 0 ? normalizeText(text) : normalizeCodeText(text);

    const tag = m[0];
    const lower = tag.toLowerCase();
    if (
      lower.startsWith('<code') ||
      lower.startsWith('<pre') ||
      lower.startsWith('<script') ||
      lower.startsWith('<style')
    ) {
      codeDepth += 1;
    } else if (
      lower.startsWith('</code') ||
      lower.startsWith('</pre') ||
      lower.startsWith('</script') ||
      lower.startsWith('</style')
    ) {
      codeDepth = Math.max(0, codeDepth - 1);
    }

    out += normalizeTag(tag);
    last = m.index + m[0].length;
  }
  let tail = noComments.slice(last);
  out += codeDepth === 0 ? normalizeText(tail) : normalizeCodeText(tail);
  return out.trim();
}

function stripTags(text) {
  // 1. Extract code block contents to preserve whitespace
  const codeBlocks = [];
  let processed = text.replace(/<code\b[^>]*>([\s\S]*?)<\/code>/gi, (_, content) => {
    codeBlocks.push(content);
    return `__CODE_BLOCK_${codeBlocks.length - 1}__`;
  });

  // 2. Remove style/script tags entirely (non-semantic content)
  processed = processed.replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, '');
  processed = processed.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, '');

  // 3. Strip remaining tags
  const blockTagRe =
    /<\/?(?:p|div|section|article|header|footer|main|aside|nav|h[1-6]|ol|ul|li|table|thead|tbody|tfoot|tr|td|th|pre|blockquote|figure|figcaption|hr|br)(?:\s[^>]*)?>/gi;
  processed = decodeHtmlEntities(
    processed
      .replace(/<!--[\s\S]*?-->/g, '')
      .replace(blockTagRe, ' ')
      .replace(/<[^>]*>/g, '')
      .replace(/\$\$([^$]+)\$\$/g, '$1')
      .replace(/\$([^$]+)\$/g, '$1'),
  )
    .trim()
    .replace(/\s+/g, ' ');

  // 4. Restore code blocks with preserved whitespace (only normalize CRLF → LF)
  codeBlocks.forEach((code, i) => {
    processed = processed.replace(`__CODE_BLOCK_${i}__`, code.replace(/\r\n?/g, '\n'));
  });

  return processed;
}

function decodeHtmlEntities(text) {
  return text
    .replace(/&nbsp;/g, ' ')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;|&apos;/g, "'")
    .replace(/&#x([0-9a-fA-F]+);/g, (_, hex) =>
      String.fromCharCode(parseInt(hex, 16)),
    )
    .replace(/&#(\d+);/g, (_, num) =>
      String.fromCharCode(parseInt(num, 10)),
    );
}

function normalizeText(text) {
  return text.replace(/\s+/g, ' ');
}

function normalizeCodeText(text) {
  return text.replace(/\r\n?/g, '\n');
}

function normalizeTag(tag) {
  const closing = /^<\s*\/\s*([^\s>]+)\s*>/.exec(tag);
  if (closing) {
    return `</${closing[1].toLowerCase()}>`;
  }

  const open = /^<\s*([^\s/>]+)([^>]*)>/.exec(tag);
  if (!open) return tag;

  const name = open[1].toLowerCase();
  const attrPart = open[2] || '';
  const attrs = [];
  const attrRe =
    /([^\s=\/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/g;
  let m;
  while ((m = attrRe.exec(attrPart))) {
    const key = m[1].toLowerCase();
    let val = m[2] ?? m[3] ?? m[4] ?? '';
    // Strip Astro-generated scoped CSS class names (astro-xxxxxxxx)
    if (key === 'class') {
      val = val
        .split(/\s+/)
        .filter((c) => !/^astro-[a-z0-9]+$/i.test(c))
        .join(' ');
    }
    // Normalize generated tab IDs (tab-XXXX, tab-panel-XXXX)
    if (key === 'id' || key === 'href' || key === 'aria-controls' || key === 'aria-labelledby') {
      val = val.replace(/\btab-panel-\d+\b/g, 'tab-panel-N');
      val = val.replace(/\btab-\d+\b/g, 'tab-N');
      // Also normalize href anchors
      val = val.replace(/#tab-panel-\d+/g, '#tab-panel-N');
      val = val.replace(/#tab-\d+/g, '#tab-N');
    }
    attrs.push([key, val]);
  }
  attrs.sort((a, b) => a[0].localeCompare(b[0]));
  const attrStr = attrs
    .map(([k, v]) => (v === '' ? k : `${k}="${v}"`))
    .join(' ');
  const selfClosing = /\/\s*>$/.test(tag);
  return `<${name}${attrStr ? ' ' + attrStr : ''}${selfClosing ? '/>' : '>'}`;
}

function summarizeDiff(a, b) {
  const maxPreview = 80;
  const min = Math.min(a.length, b.length);
  let idx = 0;
  while (idx < min && a[idx] === b[idx]) idx++;
  const start = Math.max(0, idx - 30);
  const endA = Math.min(a.length, idx + maxPreview);
  const endB = Math.min(b.length, idx + maxPreview);
  return {
    diffIndex: idx,
    previewA: a.slice(start, endA),
    previewB: b.slice(start, endB),
  };
}

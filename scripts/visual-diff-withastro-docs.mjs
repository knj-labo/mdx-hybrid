#!/usr/bin/env node
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { promises as fs } from 'node:fs';
import { dirname, extname, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';
import pixelmatch from 'pixelmatch';
import { PNG } from 'pngjs';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const withastroRoot = resolve(
  repoRoot,
  'fixtures/integration/withastro-docs/repo',
);

const DEFAULTS = {
  baseline: resolve(withastroRoot, 'dist-baseline'),
  markflow: resolve(withastroRoot, 'dist-markflow'),
  out: resolve(repoRoot, 'fixtures/integration/withastro-docs/visual-diff'),
  port: 0,
  width: 1280,
  height: 720,
  max: 40,
  threshold: 0.1,
  maxDiff: 0.002,
  wait: 200,
  build: false,
  stratifyLocales: true,
};

const CONTENT_TYPES = {
  '.html': 'text/html; charset=utf-8',
  '.css': 'text/css; charset=utf-8',
  '.js': 'text/javascript; charset=utf-8',
  '.mjs': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.svg': 'image/svg+xml',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.webp': 'image/webp',
  '.gif': 'image/gif',
  '.ico': 'image/x-icon',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
  '.ttf': 'font/ttf',
  '.otf': 'font/otf',
  '.map': 'application/json; charset=utf-8',
  '.xml': 'application/xml; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
};

const ANIMATION_KILLER = `
*,
*::before,
*::after {
  animation: none !important;
  transition: none !important;
  scroll-behavior: auto !important;
  caret-color: transparent !important;
}
`;

function usage() {
  return `
Usage: node scripts/visual-diff-withastro-docs.mjs [options]

Options:
  --build                 Build baseline/markflow outputs before diff.
  --baseline <dir>         Baseline dist directory (default: dist-baseline).
  --markflow <dir>         Markflow dist directory (default: dist-markflow).
  --out <dir>              Output directory for screenshots/diffs.
  --routes <file>          JSON array or newline list of routes.
  --include <regex>        Only include matching routes.
  --exclude <regex>        Exclude matching routes.
  --max <n>                Max routes to compare (0 = all).
  --no-stratify-locales    Disable per-locale sampling when --max is set.
  --port <n>               Base port for static servers.
  --width <n>              Viewport width.
  --height <n>             Viewport height.
  --threshold <n>          pixelmatch threshold (0-1).
  --max-diff <n>           Max diff ratio before failing.
  --wait <ms>              Extra wait after load before screenshot.
  --help                   Show this help.
`;
}

function parseArgs(argv) {
  const args = {
    baseline: DEFAULTS.baseline,
    markflow: DEFAULTS.markflow,
    out: DEFAULTS.out,
    routes: null,
    include: null,
    exclude: null,
    port: DEFAULTS.port,
    width: DEFAULTS.width,
    height: DEFAULTS.height,
    max: DEFAULTS.max,
    threshold: DEFAULTS.threshold,
    maxDiff: DEFAULTS.maxDiff,
    wait: DEFAULTS.wait,
    build: DEFAULTS.build,
    stratifyLocales: DEFAULTS.stratifyLocales,
  };

  for (let i = 2; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === '--help') {
      console.log(usage());
      process.exit(0);
    }
    if (arg === '--build') {
      args.build = true;
      continue;
    }
    if (arg === '--baseline' && argv[i + 1]) {
      args.baseline = resolve(repoRoot, argv[++i]);
      continue;
    }
    if (arg === '--markflow' && argv[i + 1]) {
      args.markflow = resolve(repoRoot, argv[++i]);
      continue;
    }
    if (arg === '--out' && argv[i + 1]) {
      args.out = resolve(repoRoot, argv[++i]);
      continue;
    }
    if (arg === '--routes' && argv[i + 1]) {
      args.routes = argv[++i];
      continue;
    }
    if (arg === '--include' && argv[i + 1]) {
      args.include = toRegex(argv[++i], '--include');
      continue;
    }
    if (arg === '--exclude' && argv[i + 1]) {
      args.exclude = toRegex(argv[++i], '--exclude');
      continue;
    }
    if (arg === '--max' && argv[i + 1]) {
      args.max = Number(argv[++i]);
      continue;
    }
    if (arg === '--no-stratify-locales') {
      args.stratifyLocales = false;
      continue;
    }
    if (arg === '--port' && argv[i + 1]) {
      args.port = Number(argv[++i]);
      continue;
    }
    if (arg === '--width' && argv[i + 1]) {
      args.width = Number(argv[++i]);
      continue;
    }
    if (arg === '--height' && argv[i + 1]) {
      args.height = Number(argv[++i]);
      continue;
    }
    if (arg === '--threshold' && argv[i + 1]) {
      args.threshold = Number(argv[++i]);
      continue;
    }
    if (arg === '--max-diff' && argv[i + 1]) {
      args.maxDiff = Number(argv[++i]);
      continue;
    }
    if (arg === '--wait' && argv[i + 1]) {
      args.wait = Number(argv[++i]);
      continue;
    }
  }

  return args;
}

function toRegex(value, flag) {
  try {
    return new RegExp(value);
  } catch (error) {
    console.error(`${flag} failed to parse regex: ${value}`);
    console.error(error.message || error);
    process.exit(1);
  }
}

async function main() {
  const opts = parseArgs(process.argv);

  if (opts.build) {
    await buildOutputs();
  }

  if (!(await exists(opts.baseline))) {
    console.error(`Baseline dist not found: ${opts.baseline}`);
    process.exit(1);
  }
  if (!(await exists(opts.markflow))) {
    console.error(`Markflow dist not found: ${opts.markflow}`);
    process.exit(1);
  }

  await ensureDir(opts.out);
  const routes = opts.routes
    ? await loadRoutes(opts.routes)
    : await collectRoutes(opts.baseline);
  const locales = await loadLocales();
  const selected = selectRoutes(routes, opts, locales);
  if (selected.length === 0) {
    console.error('No routes to compare.');
    process.exit(1);
  }

  const baselineServer = await startServer(opts.baseline, opts.port);
  const markflowServer = await startServer(
    opts.markflow,
    opts.port ? opts.port + 1 : 0,
  );

  const baseUrl = `http://127.0.0.1:${baselineServer.port}`;
  const markUrl = `http://127.0.0.1:${markflowServer.port}`;

  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: opts.width, height: opts.height },
    colorScheme: 'light',
    reducedMotion: 'reduce',
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();

  const outBase = resolve(opts.out, 'baseline');
  const outMark = resolve(opts.out, 'markflow');
  const outDiff = resolve(opts.out, 'diff');
  await Promise.all([ensureDir(outBase), ensureDir(outMark), ensureDir(outDiff)]);

  const results = [];
  let failures = 0;

  try {
  for (const route of selected) {
      const slug = routeToSlug(route);
      const basePath = resolve(outBase, `${slug}.png`);
      const markPath = resolve(outMark, `${slug}.png`);
      const diffPath = resolve(outDiff, `${slug}.png`);

      const baselineResult = await capture(page, `${baseUrl}${route}`, {
        path: basePath,
        wait: opts.wait,
      });
      const markflowResult = await capture(page, `${markUrl}${route}`, {
        path: markPath,
        wait: opts.wait,
      });

      const comparison = await compareImages(basePath, markPath, diffPath, {
        threshold: opts.threshold,
      });
      const failed =
        baselineResult.status >= 400 ||
        markflowResult.status >= 400 ||
        comparison.ratio > opts.maxDiff;

      if (failed) failures += 1;

      results.push({
        route,
        baselineStatus: baselineResult.status,
        markflowStatus: markflowResult.status,
        width: comparison.width,
        height: comparison.height,
        diffPixels: comparison.diffPixels,
        ratio: comparison.ratio,
        baseline: relative(repoRoot, basePath),
        markflow: relative(repoRoot, markPath),
        diff: relative(repoRoot, diffPath),
        failed,
      });

      console.log(
        `[visual-diff] ${route} diff=${(comparison.ratio * 100).toFixed(2)}%`,
      );
    }
  } finally {
    await browser.close();
    await stopServer(baselineServer);
    await stopServer(markflowServer);
  }

  const summary = {
    timestamp: new Date().toISOString(),
    baseline: relative(repoRoot, opts.baseline),
    markflow: relative(repoRoot, opts.markflow),
    out: relative(repoRoot, opts.out),
    routes: selected.length,
    failures,
    maxDiff: opts.maxDiff,
    threshold: opts.threshold,
  };

  await fs.writeFile(
    resolve(opts.out, 'summary.json'),
    JSON.stringify({ summary, results }, null, 2),
  );

  if (failures > 0) {
    console.error(
      `[visual-diff] ${failures} route(s) exceeded diff threshold or failed.`,
    );
    process.exitCode = 1;
  } else {
    console.log('[visual-diff] OK');
  }
}

async function buildOutputs() {
  await cleanBuildArtifacts();
  await runBuild(true);
  await moveDist('dist-baseline');
  await cleanBuildArtifacts();
  await runBuild(false);
  await moveDist('dist-markflow');
}

async function runBuild(isBaseline) {
  const env = {
    ...process.env,
    MARKFLOW_HARNESS_OFFLINE: '1',
  };
  if (!Object.prototype.hasOwnProperty.call(env, 'MARKFLOW_HARNESS_SKIP_FRONTMATTER')) {
    env.MARKFLOW_HARNESS_SKIP_FRONTMATTER = '1';
  }
  if (isBaseline) {
    env.MARKFLOW_HARNESS_BASELINE = '1';
  } else {
    delete env.MARKFLOW_HARNESS_BASELINE;
  }

  await exec('pnpm', ['--dir', withastroRoot, 'build'], env);
}

async function moveDist(targetDir) {
  const dist = resolve(withastroRoot, 'dist');
  const target = resolve(withastroRoot, targetDir);
  await fs.rm(target, { recursive: true, force: true });
  await fs.rename(dist, target);
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
    .map(normalizeRoute);
}

async function collectRoutes(rootDir) {
  const routes = [];
  await walk(rootDir, (file) => {
    if (!file.endsWith('.html')) return;
    const rel = relative(rootDir, file).split('\\').join('/');
    const route = routeFromHtml(rel);
    if (route) routes.push(route);
  });
  return Array.from(new Set(routes));
}

function selectRoutes(routes, opts, locales) {
  let out = routes;
  if (opts.include) out = out.filter((route) => opts.include.test(route));
  if (opts.exclude) out = out.filter((route) => !opts.exclude.test(route));
  out.sort((a, b) => (a === '/' ? -1 : b === '/' ? 1 : a.localeCompare(b)));
  if (opts.max === 0) return out;
  if (!opts.stratifyLocales || locales.length === 0) {
    return out.slice(0, opts.max);
  }
  return stratifyByLocale(out, locales, opts.max);
}

function stratifyByLocale(routes, locales, max) {
  const localeList = [...locales].sort((a, b) => b.length - a.length);
  const buckets = new Map();
  for (const route of routes) {
    const locale = matchLocale(route, localeList);
    const key = locale || '__root__';
    if (!buckets.has(key)) buckets.set(key, []);
    buckets.get(key).push(route);
  }

  const keys = Array.from(buckets.keys()).sort((a, b) => {
    if (a === '__root__') return -1;
    if (b === '__root__') return 1;
    return a.localeCompare(b);
  });

  const picked = [];
  let madeProgress = true;
  while (picked.length < max && madeProgress) {
    madeProgress = false;
    for (const key of keys) {
      if (picked.length >= max) break;
      const bucket = buckets.get(key);
      if (!bucket || bucket.length === 0) continue;
      picked.push(bucket.shift());
      madeProgress = true;
    }
  }
  return picked;
}

function matchLocale(route, locales) {
  for (const locale of locales) {
    const prefix = `/${locale}`;
    if (route === prefix || route.startsWith(`${prefix}/`)) {
      return locale;
    }
  }
  return null;
}

async function loadLocales() {
  const localePath = resolve(withastroRoot, 'config', 'locales.ts');
  try {
    const data = await fs.readFile(localePath, 'utf8');
    const start = data.indexOf('export const localesConfig');
    if (start === -1) return [];
    const slice = data.slice(start);
    const end = slice.indexOf('} satisfies');
    const block = end === -1 ? slice : slice.slice(0, end);
    const locales = new Set();
    const regex = /^\s*['"]?([a-z0-9-]+)['"]?:\s*{[^}]*}\s*,?$/gim;
    let match;
    while ((match = regex.exec(block))) {
      locales.add(match[1]);
    }
    return Array.from(locales);
  } catch {
    return [];
  }
}

function routeFromHtml(rel) {
  if (rel === 'index.html') return '/';
  if (rel.endsWith('/index.html')) {
    const base = rel.slice(0, -'/index.html'.length);
    return normalizeRoute(`/${base}/`);
  }
  if (rel.endsWith('.html')) {
    return normalizeRoute(`/${rel.slice(0, -'.html'.length)}`);
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

function routeToSlug(route) {
  if (route === '/') return 'index';
  return route.replace(/^\//, '').replace(/\/$/, '').replace(/\//g, '__');
}

async function walk(dir, onFile) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  for (const entry of entries) {
    const full = resolve(dir, entry.name);
    if (entry.isDirectory()) {
      await walk(full, onFile);
    } else if (entry.isFile()) {
      onFile(full);
    }
  }
}

async function capture(page, url, { path, wait }) {
  const response = await page.goto(url, { waitUntil: 'networkidle' });
  await page.addStyleTag({ content: ANIMATION_KILLER });
  await page.evaluate(async () => {
    if (document.fonts?.ready) {
      await document.fonts.ready;
    }
  });
  if (wait > 0) {
    await page.waitForTimeout(wait);
  }
  await page.screenshot({ path, fullPage: true });
  return { status: response?.status() ?? 0 };
}

async function startServer(rootDir, port) {
  const root = resolve(rootDir);
  const rootPrefix = root.endsWith(sep) ? root : `${root}${sep}`;

  const server = createServer(async (req, res) => {
    try {
      const url = new URL(req.url ?? '/', 'http://localhost');
      const pathname = decodeURIComponent(url.pathname);
      const candidate = await resolveAsset(root, pathname);
      if (!candidate) {
        res.statusCode = 404;
        res.end('Not Found');
        return;
      }
      if (!candidate.startsWith(rootPrefix)) {
        res.statusCode = 403;
        res.end('Forbidden');
        return;
      }

      const data = await fs.readFile(candidate);
      const contentType = CONTENT_TYPES[extname(candidate)] || 'application/octet-stream';
      res.setHeader('Content-Type', contentType);
      res.statusCode = 200;
      res.end(data);
    } catch (error) {
      res.statusCode = 500;
      res.end('Internal Server Error');
    }
  });

  await new Promise((resolvePromise, reject) => {
    server.listen(port, '127.0.0.1', () => resolvePromise());
    server.on('error', reject);
  });

  const address = server.address();
  return {
    server,
    port: typeof address === 'object' ? address.port : port,
  };
}

async function stopServer(handle) {
  if (!handle?.server) return;
  await new Promise((resolvePromise) => {
    handle.server.close(() => resolvePromise());
  });
}

async function resolveAsset(root, pathname) {
  const safePath = pathname.replace(/\0/g, '');
  const withPrefix = safePath.startsWith('/') ? safePath : `/${safePath}`;
  const resolved = resolve(root, `.${withPrefix}`);
  const candidates = [];

  candidates.push(resolved);
  if (!resolved.endsWith('.html')) {
    candidates.push(resolve(resolved, 'index.html'));
    candidates.push(`${resolved}.html`);
  }
  if (safePath.endsWith('/')) {
    candidates.push(resolve(root, `.${safePath}index.html`));
  }

  for (const candidate of candidates) {
    if (await exists(candidate)) {
      const stat = await fs.stat(candidate);
      if (stat.isFile()) return candidate;
    }
  }
  return null;
}

async function exists(path) {
  return fs
    .stat(path)
    .then(() => true)
    .catch(() => false);
}

async function ensureDir(path) {
  await fs.mkdir(path, { recursive: true });
}

async function compareImages(basePath, markPath, diffPath, { threshold }) {
  const basePng = PNG.sync.read(await fs.readFile(basePath));
  const markPng = PNG.sync.read(await fs.readFile(markPath));
  const width = Math.max(basePng.width, markPng.width);
  const height = Math.max(basePng.height, markPng.height);

  const baseNormalized = padImage(basePng, width, height);
  const markNormalized = padImage(markPng, width, height);

  const diff = new PNG({ width, height });
  const diffPixels = pixelmatch(
    baseNormalized.data,
    markNormalized.data,
    diff.data,
    width,
    height,
    { threshold },
  );

  await fs.writeFile(diffPath, PNG.sync.write(diff));

  return {
    width,
    height,
    diffPixels,
    ratio: diffPixels / (width * height),
  };
}

function padImage(image, width, height) {
  if (image.width === width && image.height === height) {
    return image;
  }
  const padded = new PNG({ width, height });
  padded.data.fill(255);
  for (let y = 0; y < image.height; y++) {
    for (let x = 0; x < image.width; x++) {
      const src = (image.width * y + x) * 4;
      const dst = (width * y + x) * 4;
      padded.data[dst] = image.data[src];
      padded.data[dst + 1] = image.data[src + 1];
      padded.data[dst + 2] = image.data[src + 2];
      padded.data[dst + 3] = image.data[src + 3];
    }
  }
  return padded;
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error.message || error);
    process.exit(1);
  });
}

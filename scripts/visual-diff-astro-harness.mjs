#!/usr/bin/env node
import { promises as fs } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';
import pixelmatch from 'pixelmatch';
import { PNG } from 'pngjs';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const harnessRoot = resolve(repoRoot, 'fixtures/integration/astro-harness');

const DEFAULTS = {
  width: 1280,
  tileHeight: 800,
  wait: 200,
  baseline: resolve(harnessRoot, 'dist-baseline/index.html'),
  markflow: resolve(harnessRoot, 'dist-markflow/index.html'),
  outDir: resolve(harnessRoot, 'visual-diff'),
  threshold: 0.1,
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

async function main() {
  const opts = DEFAULTS;
  const [baselineHtml, markflowHtml] = await Promise.all([
    fs.readFile(opts.baseline, 'utf8'),
    fs.readFile(opts.markflow, 'utf8'),
  ]);

  await fs.mkdir(opts.outDir, { recursive: true });

  const browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: opts.width, height: opts.tileHeight },
    colorScheme: 'light',
    reducedMotion: 'reduce',
    deviceScaleFactor: 1,
  });
  const page = await context.newPage();

  try {
    const baselineImage = await renderFullPage(page, baselineHtml, opts);
    const markflowImage = await renderFullPage(page, markflowHtml, opts);

    const baselinePath = resolve(opts.outDir, 'baseline.png');
    const markflowPath = resolve(opts.outDir, 'markflow.png');
    const diffPath = resolve(opts.outDir, 'diff.png');

    await fs.writeFile(baselinePath, PNG.sync.write(baselineImage));
    await fs.writeFile(markflowPath, PNG.sync.write(markflowImage));

    const comparison = compareImages(baselineImage, markflowImage, {
      threshold: opts.threshold,
    });
    await fs.writeFile(diffPath, PNG.sync.write(comparison.diff));

    console.log(
      JSON.stringify(
        {
          width: comparison.width,
          height: comparison.height,
          diffPixels: comparison.diffPixels,
          ratio: comparison.ratio,
          baseline: baselinePath,
          markflow: markflowPath,
          diff: diffPath,
        },
        null,
        2,
      ),
    );
  } finally {
    await browser.close();
  }
}

async function renderFullPage(page, html, opts) {
  await page.setViewportSize({ width: opts.width, height: opts.tileHeight });
  await page.setContent(html, { waitUntil: 'load' });
  await page.addStyleTag({ content: ANIMATION_KILLER });
  await page.evaluate(async () => {
    if (document.fonts?.ready) {
      await document.fonts.ready;
    }
  });
  if (opts.wait > 0) {
    await page.waitForTimeout(opts.wait);
  }

  const fullHeight = await page.evaluate(() =>
    Math.max(
      document.documentElement.scrollHeight,
      document.body?.scrollHeight ?? 0,
      document.documentElement.offsetHeight,
      document.body?.offsetHeight ?? 0,
      document.documentElement.clientHeight,
    ),
  );
  const height = Math.max(1, fullHeight);
  const tileHeight = Math.max(1, opts.tileHeight);
  const tiles = Math.max(1, Math.ceil(height / tileHeight));
  const output = new PNG({ width: opts.width, height });

  for (let i = 0; i < tiles; i++) {
    const y = Math.min(i * tileHeight, Math.max(0, height - tileHeight));
    await page.evaluate((offset) => window.scrollTo(0, offset), y);
    await page.waitForTimeout(50);
    const buffer = await page.screenshot({ type: 'png' });
    const shot = PNG.sync.read(buffer);
    const effectiveHeight = Math.min(tileHeight, height - y);
    blit(shot, output, 0, y, opts.width, effectiveHeight);
  }

  return output;
}

function compareImages(baseImage, markImage, { threshold }) {
  const width = Math.max(baseImage.width, markImage.width);
  const height = Math.max(baseImage.height, markImage.height);
  const baseNormalized = padImage(baseImage, width, height);
  const markNormalized = padImage(markImage, width, height);
  const diff = new PNG({ width, height });
  const diffPixels = pixelmatch(
    baseNormalized.data,
    markNormalized.data,
    diff.data,
    width,
    height,
    { threshold },
  );
  return {
    width,
    height,
    diffPixels,
    ratio: diffPixels / (width * height),
    diff,
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

function blit(source, target, xOffset, yOffset, width, height) {
  const safeWidth = Math.min(width, source.width, target.width);
  const safeHeight = Math.min(
    height,
    source.height,
    Math.max(0, target.height - yOffset),
  );
  for (let y = 0; y < safeHeight; y++) {
    for (let x = 0; x < safeWidth; x++) {
      const src = (source.width * y + x) * 4;
      const dst = (target.width * (yOffset + y) + (xOffset + x)) * 4;
      target.data[dst] = source.data[src];
      target.data[dst + 1] = source.data[src + 1];
      target.data[dst + 2] = source.data[src + 2];
      target.data[dst + 3] = source.data[src + 3];
    }
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  main().catch((error) => {
    console.error(error.message || error);
    process.exit(1);
  });
}

import { chromium } from 'playwright';
import fs from 'fs/promises';
import path from 'path';

const PAGES = [
  '/en/getting-started/',
  '/en/concepts/why-astro/',
  '/en/concepts/islands/',
  '/en/install-and-setup/',
  '/en/develop-and-build/',
  '/en/tutorial/0-introduction/',
  '/en/guides/middleware/',
  '/en/guides/environment-variables/',
  '/en/basics/project-structure/',
  '/en/editor-setup/',
];

const LOCAL_BASE = 'http://localhost:4323';
const PROD_BASE = 'https://docs.astro.build';
const OUTPUT_DIR = './visual-diff-output';

async function captureScreenshots() {
  await fs.mkdir(OUTPUT_DIR, { recursive: true });

  const browser = await chromium.launch();
  const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });

  for (const pagePath of PAGES) {
    const safeName = pagePath.replace(/\//g, '_');
    const page = await context.newPage();

    // Local screenshot
    try {
      await page.goto(`${LOCAL_BASE}${pagePath}`, { waitUntil: 'networkidle' });
      await page.screenshot({
        path: path.join(OUTPUT_DIR, `local${safeName}.png`),
        fullPage: true
      });
      console.log(`✓ Local: ${pagePath}`);
    } catch (e) {
      console.log(`✗ Local: ${pagePath} - ${(e as Error).message}`);
    }

    // Production screenshot
    try {
      await page.goto(`${PROD_BASE}${pagePath}`, { waitUntil: 'networkidle' });
      await page.screenshot({
        path: path.join(OUTPUT_DIR, `prod${safeName}.png`),
        fullPage: true
      });
      console.log(`✓ Prod: ${pagePath}`);
    } catch (e) {
      console.log(`✗ Prod: ${pagePath} - ${(e as Error).message}`);
    }

    await page.close();
  }

  await browser.close();
  console.log(`\nScreenshots saved to ${OUTPUT_DIR}/`);
}

captureScreenshots();

import { chromium } from 'playwright';

const url =
  process.argv[2] ||
  process.env.URL ||
  'http://localhost:4321/en/concepts/why-astro/';

const browser = await chromium.launch();
const page = await browser.newPage();

await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 15000 });
await page.waitForTimeout(500);

const result = await page.evaluate(() => {
  const sheets = [];
  const layerStatements = [];
  const layerBlocks = [];
  const layerOrder = [];

  const addLayer = (name) => {
    if (!name || name === '(anonymous)') return;
    if (!layerOrder.includes(name)) {
      layerOrder.push(name);
    }
  };

  const parseStatementNames = (cssText) => {
    let text = cssText.replace(/^@layer\s+/, '').trim();
    if (text.endsWith(';')) {
      text = text.slice(0, -1).trim();
    }
    return text
      .split(',')
      .map((name) => name.trim())
      .filter(Boolean);
  };

  const parseBlockName = (cssText) => {
    const match = cssText.match(/^@layer\s*([^\{]*)\{/);
    if (!match) return '(anonymous)';
    const name = match[1].trim();
    return name || '(anonymous)';
  };

  const walkRules = (rules, sheetInfo, parent) => {
    if (!rules) return;
    for (const rule of rules) {
      const cssText = rule.cssText || '';
      if (cssText.startsWith('@layer')) {
        const entry = {
          sheet: sheetInfo,
          parent,
          cssText,
        };
        if (cssText.includes('{')) {
          const name = parseBlockName(cssText);
          layerBlocks.push({ ...entry, name });
          addLayer(name);
        } else {
          const names = parseStatementNames(cssText);
          layerStatements.push({ ...entry, names });
          names.forEach(addLayer);
        }
      }

      if (rule.cssRules) {
        const label = rule.conditionText
          ? `@${rule.name || 'rule'} ${rule.conditionText}`
          : rule.cssText
              ? rule.cssText.split('{')[0].trim()
              : '@rule';
        const nextParent = parent ? `${parent} > ${label}` : label;
        walkRules(rule.cssRules, sheetInfo, nextParent);
      }
    }
  };

  Array.from(document.styleSheets).forEach((sheet, index) => {
    const sheetInfo = {
      index,
      href: sheet.href || '[inline]',
      media: sheet.media && sheet.media.mediaText ? sheet.media.mediaText : '',
    };
    sheets.push(sheetInfo);

    let rules;
    try {
      rules = sheet.cssRules;
    } catch (err) {
      return;
    }

    walkRules(rules, sheetInfo, '');
  });

  return {
    sheets,
    layerStatements,
    layerBlocks,
    layerOrder,
  };
});

const lines = [];
lines.push(`# inspect-layer-order`);
lines.push(`- url: ${url}`);

lines.push(`- styleSheets:`);
if (!result.sheets.length) {
  lines.push(`  - (none)`);
} else {
  for (const sheet of result.sheets) {
    lines.push(
      `  - #${sheet.index} ${sheet.href}` +
        (sheet.media ? ` (media: ${sheet.media})` : ''),
    );
  }
}

lines.push(`- layer statements:`);
if (!result.layerStatements.length) {
  lines.push(`  - (none)`);
} else {
  for (const entry of result.layerStatements) {
    lines.push(`  - sheet: #${entry.sheet.index} ${entry.sheet.href}`);
    if (entry.parent) {
      lines.push(`    - parent: ${entry.parent}`);
    }
    lines.push(`    - cssText: ${entry.cssText}`);
    if (entry.names && entry.names.length) {
      lines.push(`    - names: ${entry.names.join(', ')}`);
    }
  }
}

lines.push(`- layer blocks:`);
if (!result.layerBlocks.length) {
  lines.push(`  - (none)`);
} else {
  for (const entry of result.layerBlocks) {
    lines.push(`  - sheet: #${entry.sheet.index} ${entry.sheet.href}`);
    if (entry.parent) {
      lines.push(`    - parent: ${entry.parent}`);
    }
    lines.push(`    - name: ${entry.name}`);
    lines.push(`    - cssText: ${entry.cssText.split('\n')[0]}`);
  }
}

lines.push(`- derived layer order (first seen):`);
if (!result.layerOrder.length) {
  lines.push(`  - (none)`);
} else {
  for (const name of result.layerOrder) {
    lines.push(`  - ${name}`);
  }
}

console.log(lines.join('\n'));

await browser.close();

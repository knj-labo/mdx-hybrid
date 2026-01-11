import { chromium } from 'playwright';

const url =
  process.argv[2] ||
  process.env.URL ||
  'http://localhost:4321/en/concepts/why-astro/';
const selector =
  process.env.SELECTOR || '.sl-markdown-content .sl-heading-wrapper';

const browser = await chromium.launch();
const page = await browser.newPage();

await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 15000 });
await page.waitForTimeout(500);

const result = await page.evaluate(({ selector }) => {
  const candidates = Array.from(document.querySelectorAll(selector));
  const target =
    candidates.find(
      (el) =>
        el.previousElementSibling &&
        el.previousElementSibling.tagName === 'P',
    ) || candidates[0];

  if (!target) {
    return { error: `No element found for selector: ${selector}` };
  }

  const computed = window.getComputedStyle(target);
  const targetInfo = {
    tagName: target.tagName,
    className: target.className,
    id: target.id,
    marginTop: computed.marginTop,
    marginBottom: computed.marginBottom,
    display: computed.display,
    lineHeight: computed.lineHeight,
    fontSize: computed.fontSize,
    prevTag: target.previousElementSibling
      ? target.previousElementSibling.tagName
      : null,
    prevClass: target.previousElementSibling
      ? target.previousElementSibling.className
      : null,
  };

  let order = 0;
  const matches = [];

  const recordRule = (rule, sheetInfo, parent) => {
    const style = rule.style;
    if (!style) return;

    const margin = style.getPropertyValue('margin');
    const marginTop = style.getPropertyValue('margin-top');
    const marginBottom = style.getPropertyValue('margin-bottom');

    if (!margin && !marginTop && !marginBottom) return;

    let matchesSelector = false;
    try {
      matchesSelector = target.matches(rule.selectorText);
    } catch (err) {
      matchesSelector = false;
    }

    if (!matchesSelector) return;

    order += 1;
    matches.push({
      order,
      selectorText: rule.selectorText,
      sheet: sheetInfo,
      parent,
      margin,
      marginPriority: style.getPropertyPriority('margin'),
      marginTop,
      marginTopPriority: style.getPropertyPriority('margin-top'),
      marginBottom,
      marginBottomPriority: style.getPropertyPriority('margin-bottom'),
    });
  };

  const walkRules = (rules, sheetInfo, parent) => {
    if (!rules) return;
    for (const rule of rules) {
      if (rule.type === CSSRule.STYLE_RULE) {
        recordRule(rule, sheetInfo, parent);
        continue;
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

  for (const sheet of document.styleSheets) {
    let rules;
    try {
      rules = sheet.cssRules;
    } catch (err) {
      continue;
    }
    const sheetInfo = sheet.href || '[inline]';
    walkRules(rules, sheetInfo, '');
  }

  const last = matches.length ? matches[matches.length - 1] : null;

  return {
    targetInfo,
    matches,
    last,
  };
}, { selector });

if (result.error) {
  console.error(result.error);
  await browser.close();
  process.exit(1);
}

const lines = [];
lines.push(`# inspect-heading-margin`);
lines.push(`- url: ${url}`);
lines.push(`- selector: ${selector}`);
lines.push(`- target:`);
lines.push(`  - tag: ${result.targetInfo.tagName}`);
lines.push(`  - class: ${result.targetInfo.className}`);
lines.push(`  - id: ${result.targetInfo.id || '(none)'}`);
lines.push(`  - prevTag: ${result.targetInfo.prevTag || '(none)'}`);
lines.push(`  - prevClass: ${result.targetInfo.prevClass || '(none)'}`);
lines.push(`  - display: ${result.targetInfo.display}`);
lines.push(`  - fontSize: ${result.targetInfo.fontSize}`);
lines.push(`  - lineHeight: ${result.targetInfo.lineHeight}`);
lines.push(`  - marginTop: ${result.targetInfo.marginTop}`);
lines.push(`  - marginBottom: ${result.targetInfo.marginBottom}`);

lines.push(`- matched rules:`);
if (!result.matches.length) {
  lines.push(`  - (none)`);
} else {
  for (const rule of result.matches) {
    lines.push(`  - #${rule.order} ${rule.selectorText}`);
    lines.push(`    - sheet: ${rule.sheet}`);
    if (rule.parent) {
      lines.push(`    - parent: ${rule.parent}`);
    }
    if (rule.margin) {
      const priority = rule.marginPriority ? ` !${rule.marginPriority}` : '';
      lines.push(`    - margin: ${rule.margin}${priority}`);
    }
    if (rule.marginTop) {
      const priority = rule.marginTopPriority
        ? ` !${rule.marginTopPriority}`
        : '';
      lines.push(`    - margin-top: ${rule.marginTop}${priority}`);
    }
    if (rule.marginBottom) {
      const priority = rule.marginBottomPriority
        ? ` !${rule.marginBottomPriority}`
        : '';
      lines.push(`    - margin-bottom: ${rule.marginBottom}${priority}`);
    }
  }
}

if (result.last) {
  lines.push(`- last matching rule:`);
  lines.push(`  - #${result.last.order} ${result.last.selectorText}`);
  lines.push(`  - sheet: ${result.last.sheet}`);
  if (result.last.parent) {
    lines.push(`  - parent: ${result.last.parent}`);
  }
  if (result.last.margin) {
    lines.push(`  - margin: ${result.last.margin}`);
  }
  if (result.last.marginTop) {
    lines.push(`  - margin-top: ${result.last.marginTop}`);
  }
  if (result.last.marginBottom) {
    lines.push(`  - margin-bottom: ${result.last.marginBottom}`);
  }
}

console.log(lines.join('\n'));

await browser.close();

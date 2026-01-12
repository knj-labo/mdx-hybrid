/**
 * @file Source validation and bypass detection utilities
 * @module utils/validation
 */

/**
 * Strips headings metadata from code for component scanning
 * Removes export const headings and export function getHeadings
 * to avoid false positive component matches in metadata
 * @param {string} code - The code to process
 * @returns {string} Code without headings metadata
 */
export function stripHeadingsMeta(code) {
  return code
    .replace(/export const headings\s*=\s*\[[\s\S]*?\];\r?\n/g, "")
    .replace(/export function getHeadings\(\)\s*\{[\s\S]*?\}\r?\n/g, "");
}

/**
 * Checks if markdown source has an unclosed code fence
 * Code fences are ``` or ~~~ with 3+ characters
 * @param {string} source - The markdown source to check
 * @returns {boolean} True if there's an unclosed fence
 */
export function hasUnclosedFence(source) {
  const lines = source.split(/\r?\n/);
  let fenceChar = null;
  let fenceLength = 0;
  for (const line of lines) {
    const match = line.match(/^ {0,3}(`{3,}|~{3,})(.*)$/);
    if (!match) continue;
    const fence = match[1];
    const char = fence[0];
    const length = fence.length;
    if (!fenceChar) {
      fenceChar = char;
      fenceLength = length;
      continue;
    }
    if (char === fenceChar && length >= fenceLength) {
      fenceChar = null;
      fenceLength = 0;
    }
  }
  return Boolean(fenceChar);
}

/**
 * Determines if source should bypass Markflow and fallback to Astro MDX
 * Returns a reason string if bypass is needed, null otherwise
 * @param {string} source - The markdown/MDX source code
 * @returns {string|null} Bypass reason or null if should not bypass
 */
export function shouldBypassSource(source) {
  if (/`<\s*(Code|Prism)\b/.test(source)) {
    return "inline component code sample";
  }
  if (hasUnclosedFence(source)) {
    return "unclosed code fence";
  }
  return null;
}

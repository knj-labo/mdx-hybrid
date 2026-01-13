/**
 * @file Source validation utilities
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

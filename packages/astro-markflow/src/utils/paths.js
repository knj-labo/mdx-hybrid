/**
 * @file Path manipulation and URL derivation utilities
 * @module utils/paths
 */

import path from "node:path";

/**
 * Default file extensions that should be compiled
 */
export const DEFAULT_EXTENSIONS = new Set([".md", ".mdx"]);

/**
 * Removes query string from a file ID/path
 * @param {string} id - The file ID or path
 * @returns {string} The path without query string
 */
export function stripQuery(id) {
  if (!id) return id;
  const queryIndex = id.indexOf("?");
  return queryIndex >= 0 ? id.slice(0, queryIndex) : id;
}

/**
 * Normalizes path separators to forward slashes (Unix-style)
 * @param {string} value - The path to normalize
 * @returns {string} Path with forward slashes
 */
export function normalizePath(value) {
  return value.split(path.sep).join("/");
}

/**
 * Derives the Astro page URL from a file path
 * Converts file system paths to web URLs following Astro's routing conventions
 * @param {string} filePath - The absolute file path
 * @param {string} rootDir - The project root directory
 * @returns {string|undefined} The derived URL or undefined if not in pages directory
 */
export function deriveAstroUrl(filePath, rootDir) {
  if (!filePath) return undefined;
  const normalizedFile = normalizePath(filePath);
  const root = rootDir ?? process.cwd();
  const pagesDir = normalizePath(path.join(root, "src", "pages"));
  if (!normalizedFile.startsWith(pagesDir)) {
    return undefined;
  }
  let relative = normalizedFile.slice(pagesDir.length);
  if (relative.startsWith("/")) {
    relative = relative.slice(1);
  }
  if (!relative) {
    return "/";
  }
  if (relative.endsWith(".md") || relative.endsWith(".mdx")) {
    relative = relative.replace(/\.mdx?$/, "");
  }
  if (relative === "" || relative === "index") {
    return "/";
  }
  if (relative.endsWith("/index")) {
    relative = relative.slice(0, -"/index".length);
  }
  return `/${relative}`;
}

/**
 * Derives file options (absolute path and URL) from a Vite module ID
 * @param {string} id - The Vite module ID (may include query string)
 * @param {string} rootDir - The project root directory
 * @returns {{ file: string, url?: string }} File options with absolute path and optional URL
 */
export function deriveFileOptions(id, rootDir) {
  const sourcePath = stripQuery(id);
  let absolutePath = sourcePath;
  if (rootDir && !path.isAbsolute(sourcePath)) {
    absolutePath = path.resolve(rootDir, sourcePath);
  }
  const url = deriveAstroUrl(absolutePath, rootDir);
  const options = { file: absolutePath };
  if (url) {
    options.url = url;
  }
  return options;
}

/**
 * Checks if a file should be compiled based on its extension
 * @param {string} id - The file ID or path (may include query string)
 * @returns {boolean} True if file should be compiled
 */
export function shouldCompile(id) {
  return DEFAULT_EXTENSIONS.has(path.extname(stripQuery(id)));
}

/**
 * Configuration resolution utilities
 * @module utils/config
 */

/**
 * Default Starlight component names
 * @deprecated Use registry.getComponentsByModule('@astrojs/starlight/components') instead
 */
export const STARLIGHT_COMPONENTS = [
  "Aside",
  "Tabs",
  "TabItem",
  "Steps",
  "FileTree",
  "CardGrid",
  "LinkCard",
  "LinkButton",
  "Card",
];

/**
 * Default Starlight components module path
 * @deprecated Use starlightLibrary.defaultModulePath from markflow/registry instead
 */
export const STARLIGHT_COMPONENTS_MODULE = "@astrojs/starlight/components";

/**
 * Default ExpressiveCode component name
 * @deprecated Use expressiveCodeLibrary from markflow/registry instead
 */
export const EXPRESSIVE_CODE_COMPONENT = "ExpressiveCode";

/**
 * Default ExpressiveCode module path
 * @deprecated Use expressiveCodeLibrary.defaultModulePath from markflow/registry instead
 */
export const EXPRESSIVE_CODE_MODULE = "astro-expressive-code/components";

/**
 * Resolve ExpressiveCode configuration.
 * Normalizes boolean/object config into consistent object format.
 *
 * @param {boolean|object|null} config - User configuration
 * @returns {{ component: string, moduleId: string } | null} - Resolved config
 *
 * @example
 * resolveExpressiveCodeConfig(true)
 * // => { component: "ExpressiveCode", moduleId: "astro-expressive-code/components" }
 *
 * resolveExpressiveCodeConfig({ component: "Code", module: "my-module" })
 * // => { component: "Code", moduleId: "my-module" }
 */
export function resolveExpressiveCodeConfig(config) {
  if (!config) return null;
  if (config === true) {
    return {
      component: EXPRESSIVE_CODE_COMPONENT,
      moduleId: EXPRESSIVE_CODE_MODULE,
    };
  }
  if (typeof config === "object") {
    const component =
      typeof config.component === "string" && config.component.length > 0
        ? config.component
        : EXPRESSIVE_CODE_COMPONENT;
    const moduleId =
      typeof config.module === "string" && config.module.length > 0
        ? config.module
        : EXPRESSIVE_CODE_MODULE;
    return { component, moduleId };
  }
  return null;
}

/**
 * Resolve Starlight configuration.
 * Normalizes boolean/object config into consistent object format.
 *
 * @param {boolean|object|null} config - User configuration
 * @returns {{ components: string[], moduleId: string } | null} - Resolved config
 *
 * @example
 * resolveStarlightConfig(true)
 * // => { components: ["Aside", "Tabs", ...], moduleId: "@astrojs/starlight/components" }
 *
 * resolveStarlightConfig({ components: ["Aside"], module: "my-module" })
 * // => { components: ["Aside"], moduleId: "my-module" }
 */
export function resolveStarlightConfig(config) {
  if (!config) return null;
  if (config === true) {
    return {
      components: STARLIGHT_COMPONENTS,
      moduleId: STARLIGHT_COMPONENTS_MODULE,
    };
  }
  if (typeof config === "object") {
    const components = Array.isArray(config.components)
      ? config.components
      : STARLIGHT_COMPONENTS;
    const moduleId =
      typeof config.module === "string" && config.module.length > 0
        ? config.module
        : STARLIGHT_COMPONENTS_MODULE;
    return { components, moduleId };
  }
  return null;
}

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
 * @param {import('markflow/registry').ComponentRegistry} [registry] - Optional component registry
 * @returns {{ component: string, moduleId: string } | null} - Resolved config
 *
 * @example
 * resolveExpressiveCodeConfig(true)
 * // => { component: "ExpressiveCode", moduleId: "astro-expressive-code/components" }
 *
 * resolveExpressiveCodeConfig({ component: "Code", module: "my-module" })
 * // => { component: "Code", moduleId: "my-module" }
 *
 * // With registry (preferred):
 * resolveExpressiveCodeConfig(true, registry)
 * // Gets defaults from registry.getComponentsByModule('astro-expressive-code/components')
 */
export function resolveExpressiveCodeConfig(config, registry) {
  if (!config) return null;

  // Get defaults from registry if available, otherwise use deprecated constants
  let defaultComponent = EXPRESSIVE_CODE_COMPONENT;
  let defaultModuleId = EXPRESSIVE_CODE_MODULE;

  if (registry) {
    const ecComponents = registry.getComponentsByModule(EXPRESSIVE_CODE_MODULE);
    if (ecComponents.length > 0) {
      defaultComponent = ecComponents[0].name;
      defaultModuleId = ecComponents[0].modulePath;
    }
  }

  if (config === true) {
    return {
      component: defaultComponent,
      moduleId: defaultModuleId,
    };
  }
  if (typeof config === "object") {
    const component =
      typeof config.component === "string" && config.component.length > 0
        ? config.component
        : defaultComponent;
    const moduleId =
      typeof config.module === "string" && config.module.length > 0
        ? config.module
        : defaultModuleId;
    return { component, moduleId };
  }
  return null;
}

/**
 * Resolve Starlight configuration.
 * Normalizes boolean/object config into consistent object format.
 *
 * @param {boolean|object|null} config - User configuration
 * @param {import('markflow/registry').ComponentRegistry} [registry] - Optional component registry
 * @returns {{ components: string[], moduleId: string } | null} - Resolved config
 *
 * @example
 * resolveStarlightConfig(true)
 * // => { components: ["Aside", "Tabs", ...], moduleId: "@astrojs/starlight/components" }
 *
 * resolveStarlightConfig({ components: ["Aside"], module: "my-module" })
 * // => { components: ["Aside"], moduleId: "my-module" }
 *
 * // With registry (preferred):
 * resolveStarlightConfig(true, registry)
 * // Gets defaults from registry.getComponentsByModule('@astrojs/starlight/components')
 */
export function resolveStarlightConfig(config, registry) {
  if (!config) return null;

  // Get defaults from registry if available, otherwise use deprecated constants
  let defaultComponents = STARLIGHT_COMPONENTS;
  let defaultModuleId = STARLIGHT_COMPONENTS_MODULE;

  if (registry) {
    const slComponents = registry.getComponentsByModule(STARLIGHT_COMPONENTS_MODULE);
    if (slComponents.length > 0) {
      defaultComponents = slComponents.map((c) => c.name);
      defaultModuleId = slComponents[0].modulePath;
    }
  }

  if (config === true) {
    return {
      components: defaultComponents,
      moduleId: defaultModuleId,
    };
  }
  if (typeof config === "object") {
    const components = Array.isArray(config.components)
      ? config.components
      : defaultComponents;
    const moduleId =
      typeof config.module === "string" && config.module.length > 0
        ? config.module
        : defaultModuleId;
    return { components, moduleId };
  }
  return null;
}

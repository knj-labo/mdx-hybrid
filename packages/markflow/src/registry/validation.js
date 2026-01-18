/**
 * Registry validation utilities
 * @module registry/validation
 */

/**
 * @typedef {Object} ValidationError
 * @property {string} type - Error type ('component' | 'directive')
 * @property {string} name - Name of the invalid item
 * @property {string} message - Human-readable error message
 */

/**
 * @typedef {Object} ValidationResult
 * @property {boolean} valid - Whether the registry/library is valid
 * @property {ValidationError[]} errors - List of validation errors
 */

/**
 * Validate a component definition has required fields.
 *
 * @param {object} component - Component definition to validate
 * @returns {ValidationError[]} - Array of validation errors (empty if valid)
 */
function validateComponent(component) {
  const errors = [];

  if (!component || typeof component !== 'object') {
    errors.push({
      type: 'component',
      name: 'unknown',
      message: 'Component definition must be an object',
    });
    return errors;
  }

  if (typeof component.name !== 'string' || component.name.length === 0) {
    errors.push({
      type: 'component',
      name: component.name || 'unknown',
      message: 'Component must have a non-empty "name" string',
    });
  }

  if (typeof component.modulePath !== 'string' || component.modulePath.length === 0) {
    errors.push({
      type: 'component',
      name: component.name || 'unknown',
      message: 'Component must have a non-empty "modulePath" string',
    });
  }

  return errors;
}

/**
 * Validate a directive mapping references an existing component.
 *
 * @param {object} directive - Directive mapping to validate
 * @param {Set<string>} componentNames - Set of valid component names
 * @returns {ValidationError[]} - Array of validation errors (empty if valid)
 */
function validateDirectiveMapping(directive, componentNames) {
  const errors = [];

  if (!directive || typeof directive !== 'object') {
    errors.push({
      type: 'directive',
      name: 'unknown',
      message: 'Directive mapping must be an object',
    });
    return errors;
  }

  if (typeof directive.directive !== 'string' || directive.directive.length === 0) {
    errors.push({
      type: 'directive',
      name: directive.directive || 'unknown',
      message: 'Directive mapping must have a non-empty "directive" string',
    });
  }

  if (typeof directive.component !== 'string' || directive.component.length === 0) {
    errors.push({
      type: 'directive',
      name: directive.directive || 'unknown',
      message: 'Directive mapping must have a non-empty "component" string',
    });
  } else if (!componentNames.has(directive.component)) {
    errors.push({
      type: 'directive',
      name: directive.directive || 'unknown',
      message: `Directive mapping references unknown component "${directive.component}"`,
    });
  }

  return errors;
}

/**
 * Validate a component library preset.
 *
 * @param {object} library - Component library to validate
 * @returns {ValidationResult} - Validation result
 *
 * @example
 * import { validateLibrary } from 'markflow/registry';
 * const result = validateLibrary(starlightLibrary);
 * if (!result.valid) {
 *   console.error('Invalid library:', result.errors);
 * }
 */
export function validateLibrary(library) {
  const errors = [];

  if (!library || typeof library !== 'object') {
    return {
      valid: false,
      errors: [{
        type: 'component',
        name: 'unknown',
        message: 'Library must be an object',
      }],
    };
  }

  // Validate components
  const components = library.components ?? [];
  const componentNames = new Set();

  for (const component of components) {
    const componentErrors = validateComponent(component);
    errors.push(...componentErrors);
    if (component?.name) {
      componentNames.add(component.name);
    }
  }

  // Validate directive mappings
  const directives = library.directiveMappings ?? [];
  for (const directive of directives) {
    const directiveErrors = validateDirectiveMapping(directive, componentNames);
    errors.push(...directiveErrors);
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

/**
 * Validate a component registry.
 * Validates all components and directive mappings in the registry.
 *
 * @param {import('./index.js').ComponentRegistry} registry - Registry to validate
 * @returns {ValidationResult} - Validation result
 *
 * @example
 * import { createRegistry, validateRegistry, starlightLibrary } from 'markflow/registry';
 * const registry = createRegistry([starlightLibrary]);
 * const result = validateRegistry(registry);
 * if (!result.valid) {
 *   console.error('Invalid registry:', result.errors);
 * }
 */
export function validateRegistry(registry) {
  const errors = [];

  if (!registry || typeof registry !== 'object') {
    return {
      valid: false,
      errors: [{
        type: 'component',
        name: 'unknown',
        message: 'Registry must be an object',
      }],
    };
  }

  // Validate all components
  const allComponents = registry.getAllComponents?.() ?? [];
  const componentNames = new Set();

  for (const component of allComponents) {
    const componentErrors = validateComponent(component);
    errors.push(...componentErrors);
    if (component?.name) {
      componentNames.add(component.name);
    }
  }

  // Validate directive mappings
  const supportedDirectives = registry.getSupportedDirectives?.() ?? [];
  for (const directiveName of supportedDirectives) {
    const mapping = registry.getDirectiveMapping?.(directiveName);
    if (mapping) {
      const directiveErrors = validateDirectiveMapping(mapping, componentNames);
      errors.push(...directiveErrors);
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

import { describe, it, expect } from 'bun:test';
import {
  resolveExpressiveCodeConfig,
  resolveStarlightConfig,
  STARLIGHT_COMPONENTS,
  STARLIGHT_COMPONENTS_MODULE,
  EXPRESSIVE_CODE_COMPONENT,
  EXPRESSIVE_CODE_MODULE,
} from './config.js';

describe('resolveExpressiveCodeConfig', () => {
  it('should return null for falsy values', () => {
    expect(resolveExpressiveCodeConfig(null)).toBe(null);
    expect(resolveExpressiveCodeConfig(undefined)).toBe(null);
    expect(resolveExpressiveCodeConfig(false)).toBe(null);
  });

  it('should return default config for true', () => {
    const result = resolveExpressiveCodeConfig(true);

    expect(result).toEqual({
      component: EXPRESSIVE_CODE_COMPONENT,
      moduleId: EXPRESSIVE_CODE_MODULE,
    });
  });

  it('should use defaults for empty object', () => {
    const result = resolveExpressiveCodeConfig({});

    expect(result).toEqual({
      component: EXPRESSIVE_CODE_COMPONENT,
      moduleId: EXPRESSIVE_CODE_MODULE,
    });
  });

  it('should use custom component name', () => {
    const result = resolveExpressiveCodeConfig({
      component: 'CustomCode',
    });

    expect(result).toEqual({
      component: 'CustomCode',
      moduleId: EXPRESSIVE_CODE_MODULE,
    });
  });

  it('should use custom module path', () => {
    const result = resolveExpressiveCodeConfig({
      module: 'my-custom-module',
    });

    expect(result).toEqual({
      component: EXPRESSIVE_CODE_COMPONENT,
      moduleId: 'my-custom-module',
    });
  });

  it('should use both custom component and module', () => {
    const result = resolveExpressiveCodeConfig({
      component: 'MyCode',
      module: 'my-module',
    });

    expect(result).toEqual({
      component: 'MyCode',
      moduleId: 'my-module',
    });
  });

  it('should ignore empty string component', () => {
    const result = resolveExpressiveCodeConfig({
      component: '',
    });

    expect(result.component).toBe(EXPRESSIVE_CODE_COMPONENT);
  });

  it('should ignore empty string module', () => {
    const result = resolveExpressiveCodeConfig({
      module: '',
    });

    expect(result.moduleId).toBe(EXPRESSIVE_CODE_MODULE);
  });

  it('should ignore non-string component', () => {
    const result = resolveExpressiveCodeConfig({
      component: 123,
    });

    expect(result.component).toBe(EXPRESSIVE_CODE_COMPONENT);
  });

  it('should ignore non-string module', () => {
    const result = resolveExpressiveCodeConfig({
      module: 123,
    });

    expect(result.moduleId).toBe(EXPRESSIVE_CODE_MODULE);
  });
});

describe('resolveStarlightConfig', () => {
  it('should return null for falsy values', () => {
    expect(resolveStarlightConfig(null)).toBe(null);
    expect(resolveStarlightConfig(undefined)).toBe(null);
    expect(resolveStarlightConfig(false)).toBe(null);
  });

  it('should return default config for true', () => {
    const result = resolveStarlightConfig(true);

    expect(result).toEqual({
      components: STARLIGHT_COMPONENTS,
      moduleId: STARLIGHT_COMPONENTS_MODULE,
    });
  });

  it('should use defaults for empty object', () => {
    const result = resolveStarlightConfig({});

    expect(result).toEqual({
      components: STARLIGHT_COMPONENTS,
      moduleId: STARLIGHT_COMPONENTS_MODULE,
    });
  });

  it('should use custom components array', () => {
    const customComponents = ['Aside', 'Tabs'];
    const result = resolveStarlightConfig({
      components: customComponents,
    });

    expect(result).toEqual({
      components: customComponents,
      moduleId: STARLIGHT_COMPONENTS_MODULE,
    });
  });

  it('should use custom module path', () => {
    const result = resolveStarlightConfig({
      module: 'my-starlight-module',
    });

    expect(result).toEqual({
      components: STARLIGHT_COMPONENTS,
      moduleId: 'my-starlight-module',
    });
  });

  it('should use both custom components and module', () => {
    const customComponents = ['CustomAside'];
    const result = resolveStarlightConfig({
      components: customComponents,
      module: 'custom-module',
    });

    expect(result).toEqual({
      components: customComponents,
      moduleId: 'custom-module',
    });
  });

  it('should use default components for non-array', () => {
    const result = resolveStarlightConfig({
      components: 'not-an-array',
    });

    expect(result.components).toEqual(STARLIGHT_COMPONENTS);
  });

  it('should ignore empty string module', () => {
    const result = resolveStarlightConfig({
      module: '',
    });

    expect(result.moduleId).toBe(STARLIGHT_COMPONENTS_MODULE);
  });

  it('should ignore non-string module', () => {
    const result = resolveStarlightConfig({
      module: 123,
    });

    expect(result.moduleId).toBe(STARLIGHT_COMPONENTS_MODULE);
  });

  it('should handle empty components array', () => {
    const result = resolveStarlightConfig({
      components: [],
    });

    expect(result.components).toEqual([]);
  });
});

describe('constants', () => {
  it('should export STARLIGHT_COMPONENTS array', () => {
    expect(Array.isArray(STARLIGHT_COMPONENTS)).toBe(true);
    expect(STARLIGHT_COMPONENTS.length).toBeGreaterThan(0);
    expect(STARLIGHT_COMPONENTS).toContain('Aside');
  });

  it('should export STARLIGHT_COMPONENTS_MODULE string', () => {
    expect(typeof STARLIGHT_COMPONENTS_MODULE).toBe('string');
    expect(STARLIGHT_COMPONENTS_MODULE).toBe('@astrojs/starlight/components');
  });

  it('should export EXPRESSIVE_CODE_COMPONENT string', () => {
    expect(typeof EXPRESSIVE_CODE_COMPONENT).toBe('string');
    expect(EXPRESSIVE_CODE_COMPONENT).toBe('ExpressiveCode');
  });

  it('should export EXPRESSIVE_CODE_MODULE string', () => {
    expect(typeof EXPRESSIVE_CODE_MODULE).toBe('string');
    expect(EXPRESSIVE_CODE_MODULE).toBe('astro-expressive-code/components');
  });
});

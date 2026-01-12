/**
 * Functional pipeline utilities for composing transformations
 * @module pipeline/pipe
 */

/**
 * Compose functions into a pipeline that passes data through each step.
 * Supports both sync and async functions.
 *
 * @param {...Function} fns - Functions to compose (sync or async)
 * @returns {Function} - Composed function that runs the pipeline
 *
 * @example
 * const transform = pipe(
 *   (data) => step1(data),
 *   async (data) => await step2(data),
 *   (data) => step3(data)
 * );
 * const result = await transform(input);
 */
export function pipe(...fns) {
  return async (input) => {
    let result = input;
    for (const fn of fns) {
      result = await fn(result);
    }
    return result;
  };
}

/**
 * Create a conditional pipeline step.
 * Only executes transform if condition is met.
 *
 * @param {boolean|Function} condition - Static or dynamic condition
 * @param {Function} transform - Transform to apply if condition is true
 * @returns {Function} - Conditional transform function
 *
 * @example
 * const pipeline = pipe(
 *   when(config.enableShiki, (data) => highlightCode(data)),
 *   when((data) => data.hasComponents, (data) => injectImports(data))
 * );
 */
export function when(condition, transform) {
  return async (data) => {
    const shouldRun = typeof condition === 'function'
      ? condition(data)
      : condition;

    if (!shouldRun) {
      return data;
    }
    return await transform(data);
  };
}

/**
 * Tap into pipeline for side effects without modifying data.
 * Useful for logging, validation, or triggering watchers.
 *
 * @param {Function} sideEffect - Function to call (can be async)
 * @returns {Function} - Pass-through transform
 *
 * @example
 * const pipeline = pipe(
 *   transform1,
 *   tap((data) => console.log('After transform1:', data)),
 *   transform2
 * );
 */
export function tap(sideEffect) {
  return async (data) => {
    await sideEffect(data);
    return data;
  };
}

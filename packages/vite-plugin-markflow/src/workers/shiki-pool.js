/**
 * Shiki Worker Pool
 *
 * Manages a pool of worker threads for parallel Shiki syntax highlighting.
 * Uses worker_threads to bypass Node.js single-threaded limitations for CPU-bound tasks.
 */
import { Worker } from 'node:worker_threads';
import { cpus } from 'node:os';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const WORKER_PATH = path.join(__dirname, 'shiki-worker.js');

/**
 * @typedef {Object} PendingTask
 * @property {string} id - Unique task identifier
 * @property {string} code - Code to highlight
 * @property {string} lang - Language for highlighting
 * @property {function} resolve - Promise resolve callback
 * @property {function} reject - Promise reject callback
 */

/**
 * @typedef {Object} WorkerState
 * @property {Worker} worker - The worker thread instance
 * @property {boolean} busy - Whether the worker is currently processing
 * @property {boolean} ready - Whether the worker has signaled ready
 * @property {Map<string, PendingTask>} pending - Pending tasks for this worker
 */

export class ShikiWorkerPool {
  /** @type {WorkerState[]} */
  #workers = [];

  /** @type {PendingTask[]} */
  #queue = [];

  /** @type {number} */
  #nextTaskId = 0;

  /** @type {boolean} */
  #terminated = false;

  /** @type {Promise<void>} */
  #readyPromise;

  /**
   * Creates a new Shiki worker pool.
   * @param {number} [size] - Number of workers (defaults to CPU count)
   */
  constructor(size = Math.max(1, cpus().length - 1)) {
    const readyPromises = [];

    for (let i = 0; i < size; i++) {
      const worker = new Worker(WORKER_PATH);
      const state = {
        worker,
        busy: false,
        ready: false,
        pending: new Map(),
      };

      // Track ready promise
      const readyPromise = new Promise((resolve) => {
        const readyHandler = (msg) => {
          if (msg.type === 'ready') {
            state.ready = true;
            worker.off('message', readyHandler);
            resolve();
          }
        };
        worker.on('message', readyHandler);
      });
      readyPromises.push(readyPromise);

      // Handle responses
      worker.on('message', (msg) => {
        if (msg.type === 'ready') return;

        const { id, html, error } = msg;
        const task = state.pending.get(id);
        if (!task) return;

        state.pending.delete(id);
        state.busy = state.pending.size > 0;

        if (error) {
          task.reject(new Error(error));
        } else {
          task.resolve(html);
        }

        // Process next task in queue
        this.#processQueue();
      });

      worker.on('error', (error) => {
        // Reject all pending tasks for this worker
        for (const [, task] of state.pending) {
          task.reject(error);
        }
        state.pending.clear();
        state.busy = false;
        this.#processQueue();
      });

      this.#workers.push(state);
    }

    // Wait for all workers to be ready
    this.#readyPromise = Promise.all(readyPromises).then(() => {});
  }

  /**
   * Waits for the pool to be ready.
   * @returns {Promise<void>}
   */
  async ready() {
    return this.#readyPromise;
  }

  /**
   * Get pool size (number of workers).
   * @returns {number}
   */
  get size() {
    return this.#workers.length;
  }

  /**
   * Highlights code using an available worker.
   * @param {string} code - Code to highlight
   * @param {string} [lang] - Language for highlighting
   * @returns {Promise<string>} Highlighted HTML
   */
  async highlight(code, lang) {
    if (this.#terminated) {
      throw new Error('Worker pool has been terminated');
    }

    // Wait for workers to be ready
    await this.#readyPromise;

    const id = `task-${this.#nextTaskId++}`;

    return new Promise((resolve, reject) => {
      const task = { id, code, lang, resolve, reject };

      // Find an available worker
      const availableWorker = this.#workers.find((w) => !w.busy && w.ready);

      if (availableWorker) {
        this.#dispatchTask(availableWorker, task);
      } else {
        // Queue the task
        this.#queue.push(task);
      }
    });
  }

  /**
   * Dispatches a task to a worker.
   * @param {WorkerState} workerState
   * @param {PendingTask} task
   */
  #dispatchTask(workerState, task) {
    workerState.busy = true;
    workerState.pending.set(task.id, task);
    workerState.worker.postMessage({
      id: task.id,
      code: task.code,
      lang: task.lang,
    });
  }

  /**
   * Processes queued tasks.
   */
  #processQueue() {
    if (this.#queue.length === 0) return;

    const availableWorker = this.#workers.find((w) => !w.busy && w.ready);
    if (!availableWorker) return;

    const task = this.#queue.shift();
    if (task) {
      this.#dispatchTask(availableWorker, task);
    }
  }

  /**
   * Terminates all workers in the pool.
   * @returns {Promise<void>}
   */
  async terminate() {
    if (this.#terminated) return;
    this.#terminated = true;

    // Reject all queued tasks
    for (const task of this.#queue) {
      task.reject(new Error('Worker pool terminated'));
    }
    this.#queue = [];

    // Terminate all workers
    await Promise.all(
      this.#workers.map(async (state) => {
        // Reject pending tasks
        for (const [, task] of state.pending) {
          task.reject(new Error('Worker pool terminated'));
        }
        state.pending.clear();
        await state.worker.terminate();
      })
    );

    this.#workers = [];
  }
}

// Singleton instance for the plugin
let poolInstance = null;

/**
 * Creates or returns the singleton pool instance.
 * @param {number} [size] - Pool size (only used on first call)
 * @returns {ShikiWorkerPool}
 */
export function getShikiPool(size) {
  if (!poolInstance) {
    poolInstance = new ShikiWorkerPool(size);
  }
  return poolInstance;
}

/**
 * Terminates the singleton pool instance.
 * @returns {Promise<void>}
 */
export async function terminateShikiPool() {
  if (poolInstance) {
    await poolInstance.terminate();
    poolInstance = null;
  }
}

/**
 * Creates a highlighter function that uses the worker pool.
 * @param {ShikiWorkerPool} pool
 * @returns {(code: string, lang: string) => Promise<string>}
 */
export function createPooledHighlighter(pool) {
  return async (code, lang) => {
    return pool.highlight(code, lang);
  };
}

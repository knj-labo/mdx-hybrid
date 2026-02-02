type ProfileStats = {
  totalMs: number;
  count: number;
  maxMs: number;
};

type ProfileStore = {
  stats: Map<string, ProfileStats>;
  hooked: boolean;
  dumped: boolean;
  top: number;
};

const ENABLED =
  typeof process !== 'undefined' && process.env?.ASTRO_COMPONENT_PROFILE === '1';

const now = () =>
  globalThis.performance && typeof globalThis.performance.now === 'function'
    ? globalThis.performance.now()
    : Date.now();

const getStore = (): ProfileStore | null => {
  if (!ENABLED) return null;

  const g = globalThis as { __astroComponentProfile?: ProfileStore };
  if (!g.__astroComponentProfile) {
    const rawTop =
      typeof process !== 'undefined' && process.env?.ASTRO_COMPONENT_PROFILE_TOP
        ? Number.parseInt(process.env.ASTRO_COMPONENT_PROFILE_TOP, 10)
        : 20;
    g.__astroComponentProfile = {
      stats: new Map<string, ProfileStats>(),
      hooked: false,
      dumped: false,
      top: Number.isFinite(rawTop) && rawTop > 0 ? rawTop : 20,
    };
  }

  const store = g.__astroComponentProfile;
  if (store && !store.hooked && typeof process !== 'undefined' && typeof process.on === 'function') {
    store.hooked = true;
    const dump = () => {
      if (store.dumped) return;
      store.dumped = true;

      const entries = Array.from(store.stats.entries()).map(([label, stat]) => ({
        label,
        totalMs: stat.totalMs,
        count: stat.count,
        maxMs: stat.maxMs,
        avgMs: stat.count > 0 ? stat.totalMs / stat.count : 0,
      }));
      entries.sort((a, b) => b.totalMs - a.totalMs);

      const total = entries.reduce((acc, entry) => acc + entry.totalMs, 0);
      console.log(`[component-profiler] total=${total.toFixed(2)}ms entries=${entries.length}`);
      for (const entry of entries.slice(0, store.top)) {
        console.log(
          `[component-profiler] ${entry.label} total=${entry.totalMs.toFixed(2)}ms ` +
            `avg=${entry.avgMs.toFixed(2)}ms max=${entry.maxMs.toFixed(2)}ms n=${entry.count}`
        );
      }
    };
    process.on('beforeExit', dump);
    process.on('exit', dump);
  }

  return store ?? null;
};

export const startProfile = (label: string): number | null => {
  if (!ENABLED) return null;
  const store = getStore();
  if (!store) return null;
  return now();
};

export const endProfile = (label: string, startedAt: number | null): string => {
  if (!ENABLED || startedAt == null) return '';
  const store = getStore();
  if (!store) return '';
  const duration = now() - startedAt;
  const stat = store.stats.get(label) ?? { totalMs: 0, count: 0, maxMs: 0 };
  stat.totalMs += duration;
  stat.count += 1;
  stat.maxMs = Math.max(stat.maxMs, duration);
  store.stats.set(label, stat);
  return '';
};

export const profileScope = <T>(label: string, fn: () => T): T => {
  const start = startProfile(label);
  try {
    return fn();
  } finally {
    endProfile(label, start);
  }
};

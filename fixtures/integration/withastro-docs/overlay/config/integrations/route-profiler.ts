import type { AstroIntegration } from 'astro';

type RouteProfileStats = {
	totalMs: number;
	count: number;
	maxMs: number;
};

type RouteProfileSummary = {
	totalMs: number;
	count: number;
	routeCount: number;
	maxMs: number;
};

type PrefixProfileSummary = {
	label: string;
	totalMs: number;
	count: number;
	routeCount: number;
	maxMs: number;
	avgMs: number;
};

const PREFIX_LABELS: Array<{ label: string; prefixes: string[] }> = [
	{ label: '/open-graph', prefixes: ['/open-graph/'] },
	{ label: '/docs', prefixes: ['/en/', '/ja/', '/fr/', '/de/', '/es/', '/pt-br/', '/it/', '/ru/', '/ar/', '/hi/', '/ko/', '/zh-cn/', '/zh-tw/', '/pl/', '/tr/'] },
	{ label: '/api', prefixes: ['/api/'] },
	{ label: '/assets', prefixes: ['/assets/', '/_astro/'] },
	{ label: '/misc', prefixes: [] },
];

const ROUTE_PROFILE =
	typeof process !== 'undefined' && process.env?.ASTRO_ROUTE_PROFILE === '1';

const TOP_COUNT = (() => {
	const raw = process.env?.ASTRO_ROUTE_PROFILE_TOP;
	if (!raw) return 20;
	const parsed = Number.parseInt(raw, 10);
	return Number.isFinite(parsed) && parsed > 0 ? parsed : 20;
})();

const stripAnsi = (input: string) => input.replace(/\u001B\[[0-9;]*m/g, '');

export function routeProfiler(): AstroIntegration {
	let buffer = '';
	let lastPath: string | null = null;
	let originalWrite: typeof process.stdout.write | null = null;
	const stats = new Map<string, RouteProfileStats>();

	const record = (label: string, durationMs: number) => {
		const entry = stats.get(label) ?? { totalMs: 0, count: 0, maxMs: 0 };
		entry.totalMs += durationMs;
		entry.count += 1;
		entry.maxMs = Math.max(entry.maxMs, durationMs);
		stats.set(label, entry);
	};

	const parseLine = (line: string) => {
		const cleaned = stripAnsi(line).trimEnd();
		if (!cleaned) return;

		const pathMatch = cleaned.match(/^[^A-Za-z0-9]*[└├]─\s+(.+)$/);
		if (pathMatch) {
			lastPath = pathMatch[1].trim();
		}

		const timeMatch = cleaned.match(/\(\+([0-9.]+)(ms|s)\)/);
		if (!timeMatch) return;

		const value = Number.parseFloat(timeMatch[1]);
		const ms = timeMatch[2] === 's' ? value * 1000 : value;

		let label = lastPath;
		const inlinePathMatch = cleaned.match(/\s(\S+)\s*\(\+[0-9.]+(?:ms|s)\)/);
		if (inlinePathMatch) {
			label = inlinePathMatch[1];
		}

		if (!label) return;
		record(label, ms);
	};

	const handleWrite = (chunk: string) => {
		buffer += chunk;
		let idx = buffer.indexOf('\n');
		while (idx !== -1) {
			const line = buffer.slice(0, idx);
			buffer = buffer.slice(idx + 1);
			parseLine(line);
			idx = buffer.indexOf('\n');
		}
	};

	const flush = () => {
		if (buffer) {
			parseLine(buffer);
			buffer = '';
		}
	};

	const computeSummary = (): RouteProfileSummary => {
		let totalMs = 0;
		let count = 0;
		let maxMs = 0;
		for (const entry of stats.values()) {
			totalMs += entry.totalMs;
			count += entry.count;
			maxMs = Math.max(maxMs, entry.maxMs);
		}
		return {
			totalMs,
			count,
			routeCount: stats.size,
			maxMs,
		};
	};

	const computePrefixSummaries = (): PrefixProfileSummary[] => {
		const summaries = new Map<string, PrefixProfileSummary>();
		const ensure = (label: string): PrefixProfileSummary => {
			const existing = summaries.get(label);
			if (existing) return existing;
			const entry: PrefixProfileSummary = {
				label,
				totalMs: 0,
				count: 0,
				routeCount: 0,
				maxMs: 0,
				avgMs: 0,
			};
			summaries.set(label, entry);
			return entry;
		};

		const matchLabel = (route: string): string => {
			for (const group of PREFIX_LABELS) {
				if (group.prefixes.some(prefix => route.startsWith(prefix))) {
					return group.label;
				}
			}
			return '/misc';
		};

		for (const [route, stat] of stats.entries()) {
			const label = matchLabel(route);
			const summary = ensure(label);
			summary.totalMs += stat.totalMs;
			summary.count += stat.count;
			summary.routeCount += 1;
			summary.maxMs = Math.max(summary.maxMs, stat.maxMs);
		}

		for (const summary of summaries.values()) {
			summary.avgMs = summary.count > 0 ? summary.totalMs / summary.count : 0;
		}

		return Array.from(summaries.values()).sort((a, b) => b.totalMs - a.totalMs);
	};

	return {
		name: 'route-profiler',
		hooks: {
			'astro:build:start'() {
				if (!ROUTE_PROFILE) return;
				if (!process?.stdout?.write) return;
				originalWrite = process.stdout.write.bind(process.stdout);
				process.stdout.write = ((chunk: any, ...args: any[]) => {
					try {
						handleWrite(typeof chunk === 'string' ? chunk : chunk.toString('utf8'));
					} catch {
						// Ignore profiling parse errors.
					}
					return originalWrite?.(chunk, ...args) ?? true;
				}) as typeof process.stdout.write;
			},
			'astro:build:done'() {
				if (!ROUTE_PROFILE) return;
				flush();
				if (originalWrite) {
					process.stdout.write = originalWrite;
				}

				if (!stats.size) return;
				const summary = computeSummary();
				(globalThis as { __astroRouteProfile?: RouteProfileSummary }).__astroRouteProfile =
					summary;
				const avg = summary.count > 0 ? summary.totalMs / summary.count : 0;
				console.log(
					`[route-profiler] summary total=${summary.totalMs.toFixed(2)}ms ` +
						`avg=${avg.toFixed(2)}ms max=${summary.maxMs.toFixed(2)}ms ` +
						`routes=${summary.routeCount} renders=${summary.count}`
				);
				const prefixSummaries = computePrefixSummaries();
				if (prefixSummaries.length > 0) {
					console.log(`[route-profiler] prefix summary (total ms)`);
					for (const entry of prefixSummaries) {
						console.log(
							`[route-profiler] ${entry.label} total=${entry.totalMs.toFixed(2)}ms ` +
								`avg=${entry.avgMs.toFixed(2)}ms max=${entry.maxMs.toFixed(2)}ms ` +
								`routes=${entry.routeCount} renders=${entry.count}`
						);
					}
				}
				const entries = [...stats.entries()].sort((a, b) => b[1].totalMs - a[1].totalMs);
				console.log(`[route-profiler] top ${TOP_COUNT} routes (total ms)`);
				for (const [label, stat] of entries.slice(0, TOP_COUNT)) {
					const avg = stat.totalMs / stat.count;
					console.log(
						`[route-profiler] ${label} total=${stat.totalMs.toFixed(2)}ms ` +
							`avg=${avg.toFixed(2)}ms max=${stat.maxMs.toFixed(2)}ms n=${stat.count}`
					);
				}
			},
		},
	};
}

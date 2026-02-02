import type { AstroIntegration } from 'astro';
import { performance } from 'node:perf_hooks';

type Target = 'client' | 'server';

const now = () => performance.now();

export function buildProfiler(): AstroIntegration {
	const starts = new Map<string, number>();
	const durations = new Map<string, number>();
	let buildStartTime: number | null = null;

	const start = (label: string) => {
		starts.set(label, now());
	};

	const end = (label: string) => {
		const startedAt = starts.get(label);
		if (startedAt != null) {
			durations.set(label, now() - startedAt);
		}
	};

	const viteTimingPlugin = (target: Target) => {
		let buildStartTime = 0;
		return {
			name: `build-profiler:vite:${target}`,
			apply: 'build' as const,
			buildStart() {
				buildStartTime = now();
			},
			closeBundle() {
				if (buildStartTime > 0) {
					durations.set(`vite:${target}`, now() - buildStartTime);
				}
			},
		};
	};

	const formatMs = (value?: number) => (typeof value === 'number' ? Math.round(value) : 'n/a');

	return {
		name: 'build-profiler',
		hooks: {
			'astro:build:start'() {
				buildStartTime = now();
				start('astro:build:total');
			},
			'astro:build:setup'({ target, updateConfig }) {
				updateConfig({
					plugins: [viteTimingPlugin(target)],
				});
			},
			'astro:build:generated'() {
				if (buildStartTime != null) {
					durations.set('astro:build:toGenerated', now() - buildStartTime);
				}
			},
			'astro:build:done'() {
				end('astro:build:total');

				const total = durations.get('astro:build:total');
				const toGenerated = durations.get('astro:build:toGenerated');
				const viteClient = durations.get('vite:client') ?? 0;
				const viteServer = durations.get('vite:server') ?? 0;
				const viteTotal = viteClient + viteServer;
				const nonVite = typeof total === 'number' ? total - viteTotal : undefined;
				const afterGenerated =
					typeof total === 'number' && typeof toGenerated === 'number'
						? total - toGenerated
						: undefined;

				console.log(
					`[build-profiler] total=${formatMs(total)}ms ` +
						`vite:client=${formatMs(durations.get('vite:client'))}ms ` +
						`vite:server=${formatMs(durations.get('vite:server'))}ms ` +
						`vite:total=${formatMs(viteTotal)}ms ` +
						`non-vite=${formatMs(nonVite)}ms`
				);

				if (typeof toGenerated === 'number') {
					console.log(
						`[build-profiler] phases toGenerated=${formatMs(toGenerated)}ms ` +
							`afterGenerated=${formatMs(afterGenerated)}ms`
					);
				}

				const routeProfile = (globalThis as { __astroRouteProfile?: { totalMs: number; count: number; routeCount: number } })
					.__astroRouteProfile;
				if (routeProfile) {
					const avg = routeProfile.count > 0 ? routeProfile.totalMs / routeProfile.count : 0;
					console.log(
						`[build-profiler] routes total=${formatMs(routeProfile.totalMs)}ms ` +
							`avg=${avg.toFixed(2)}ms routes=${routeProfile.routeCount}`
					);
					if (typeof nonVite === 'number') {
						const remainder = nonVite - routeProfile.totalMs;
						const pct = nonVite > 0 ? (routeProfile.totalMs / nonVite) * 100 : 0;
						console.log(
							`[build-profiler] non-vite minus routes=${formatMs(remainder)}ms ` +
								`routesPct=${pct.toFixed(1)}%`
						);
					}
				}
			},
		},
	};
}

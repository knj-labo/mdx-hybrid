import type { AstroIntegration } from 'astro';

const ENABLED =
	typeof process !== 'undefined' && process.env?.ASTRO_VITE_TUNER === '1';

export function viteBuildTuner(): AstroIntegration {
	return {
		name: 'vite-build-tuner',
		hooks: {
			'astro:build:setup'({ target, updateConfig }) {
				if (!ENABLED || target !== 'server') return;
				updateConfig({
					build: {
						minify: false,
						cssMinify: false,
						target: 'esnext',
						reportCompressedSize: false,
					},
					esbuild: {
						target: 'esnext',
					},
				});
			},
		},
	};
}

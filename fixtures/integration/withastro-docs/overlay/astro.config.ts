import markflow from "astro-markflow";
import starlight from "@astrojs/starlight";
import { defineConfig, sharpImageService } from "astro/config";
import { cpus } from "node:os";
import rehypeSlug from "rehype-slug";
import remarkSmartypants from "remark-smartypants";
import { sidebar } from "./astro.sidebar";
import { devServerFileWatcher } from "./config/integrations/dev-server-file-watcher";
import { buildProfiler } from "./config/integrations/build-profiler";
import { routeProfiler } from "./config/integrations/route-profiler";
import { viteBuildTuner } from "./config/integrations/vite-build-tuner";
import { sitemap } from "./config/integrations/sitemap";
import { localesConfig } from "./config/locales";
import { starlightPluginSmokeTest } from "./config/plugins/smoke-test";
import { rehypeTasklistEnhancer } from "./config/plugins/rehype-tasklist-enhancer";
import { remarkFallbackLang } from "./config/plugins/remark-fallback-lang";

/* https://docs.netlify.com/configure-builds/environment-variables/#read-only-variables */
const NETLIFY_PREVIEW_SITE =
  process.env.CONTEXT !== "production" && process.env.DEPLOY_PRIME_URL;

const site = NETLIFY_PREVIEW_SITE || "https://docs.astro.build/";

// Used by CI harness to build without markflow for baseline comparison
const isBaseline = process.env.MARKFLOW_HARNESS_BASELINE === "1";
const enableBuildProfiler = process.env.ASTRO_BUILD_PROFILE === "1";
const enableRouteProfiler = process.env.ASTRO_ROUTE_PROFILE === "1";
const enableViteTuner = process.env.ASTRO_VITE_TUNER === "1";
const buildConcurrency = Math.max(1, cpus().length - 1);

// https://astro.build/config
export default defineConfig({
  site,
  build: { inlineStylesheets: "never", concurrency: buildConcurrency },
  integrations: [
    ...(enableBuildProfiler ? [buildProfiler()] : []),
    ...(enableRouteProfiler ? [routeProfiler()] : []),
    ...(enableViteTuner ? [viteBuildTuner()] : []),
    ...(isBaseline
      ? []
      : [
          markflow({
            starlightComponents: true,
            expressiveCode: true, // Use EC via ec.config.mjs for non-serializable plugins
          }),
        ]),
    devServerFileWatcher([
      "./config/**", // Custom plugins and integrations
      "./astro.sidebar.ts", // Sidebar configuration file
      "./src/content/nav/*.ts", // Sidebar labels
    ]),
    starlight({
      title: "Docs",
      expressiveCode: true,
      components: {
        Hero: "./src/components/starlight/Hero.astro",
        EditLink: "./src/components/starlight/EditLink.astro",
        MarkdownContent: "./src/components/starlight/MarkdownContent.astro",
        MobileTableOfContents:
          "./src/components/starlight/MobileTableOfContents.astro",
        TableOfContents: "./src/components/starlight/TableOfContents.astro",
        PageSidebar: "./src/components/starlight/PageSidebar.astro",
        Footer: "./src/components/starlight/Footer.astro",
        SiteTitle: "./src/components/starlight/SiteTitle.astro",
        Search: "./src/components/starlight/Search.astro",
        Sidebar: "./src/components/starlight/Sidebar.astro",
        MobileMenuFooter: "./src/components/starlight/MobileMenuFooter.astro",
        PageTitle: "./src/components/starlight/PageTitle.astro",
        Head: "./src/components/starlight/Head.astro",
      },
      routeMiddleware: "./src/routeData.ts",
      editLink: {
        baseUrl: "https://github.com/withastro/docs/edit/main",
      },
      defaultLocale: "en",
      locales: localesConfig,
      sidebar,
      social: [
        {
          icon: "github",
          label: "GitHub",
          href: "https://github.com/withastro/astro",
        },
        { icon: "discord", label: "Discord", href: "https://astro.build/chat" },
      ],
      pagefind: false,
      head: [
        // Add ICO favicon fallback for Safari.
        {
          tag: "link",
          attrs: {
            rel: "icon",
            href: "/favicon.ico",
            sizes: "32x32",
          },
        },
      ],
      disable404Route: true,
      plugins: [starlightPluginSmokeTest()],
    }),
    sitemap(),
  ],
  trailingSlash: "always",
  scopedStyleStrategy: "where",
  compressHTML: false,
  markdown: {
    // Override with our own config
    smartypants: false,
    remarkPlugins: [
      // @ts-expect-error — `remark-smartypants` type is not matching Astro’s for some reason even though they both use unified’s `Plugin` type
      [remarkSmartypants, { dashes: false }],
      // Add our custom plugin that marks links to fallback language pages
      remarkFallbackLang(),
    ],
    rehypePlugins: [
      rehypeSlug,
      // Tweak GFM task list syntax
      rehypeTasklistEnhancer(),
    ],
  },
  image: {
    domains: ["avatars.githubusercontent.com"],
    service: sharpImageService(),
  },
});

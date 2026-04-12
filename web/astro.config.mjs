// @ts-check

import sitemap from "@astrojs/sitemap";
import { defineConfig } from "astro/config";

// https://astro.build/config
export default defineConfig({
  site: "https://musicopy.app",
  integrations: [
    sitemap({
      serialize(item) {
        if (item.url.includes("/license-confirmation")) {
          return undefined;
        }
        return item;
      },
    }),
  ],
  redirects: {
    "/pricing": "/license",
  },
});

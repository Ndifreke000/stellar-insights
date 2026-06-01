import { defineRouting } from "next-intl/routing";

export const routing = defineRouting({
  locales: ["en", "es", "zh"],
  defaultLocale: "en",
  localePrefix: "always",
});

import { defineConfig, globalIgnores } from "eslint/config";
import nextVitals from "eslint-config-next/core-web-vitals";
import nextTs from "eslint-config-next/typescript";

const eslintConfig = defineConfig([
  ...nextVitals,
  ...nextTs,
  // Override default ignores of eslint-config-next.
  globalIgnores([
    // Default ignores of eslint-config-next:
    ".next/**",
    "out/**",
    "build/**",
    "next-env.d.ts",
  ]),
  {
    // Plain CommonJS Node scripts (e.g. scripts/i18n-audit.js) run via
    // `node scripts/foo.js` directly, without a build step -- they must use
    // require(), not ESM import, since this package.json has no
    // "type": "module".
    files: ["scripts/**/*.js"],
    rules: {
      "@typescript-eslint/no-require-imports": "off",
    },
  },
  {
    rules: {
      "no-console": "off",
      // Allow intentionally-unused bindings (e.g. required callback/positional
      // params, or destructured values kept for documentation) to be marked
      // with a leading underscore instead of being deleted.
      "@typescript-eslint/no-unused-vars": [
        "warn",
        {
          argsIgnorePattern: "^_",
          varsIgnorePattern: "^_",
          caughtErrorsIgnorePattern: "^_",
          enableAutofixRemoval: { imports: true },
        },
      ],
    },
  },
]);

export default eslintConfig;

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
      // react-hooks/set-state-in-effect and react-hooks/purity are
      // experimental React Compiler rules (eslint-plugin-react-hooks's
      // `recommended` config, spread into eslint-config-next). Their static
      // analysis is transitive and coarse: it flags any effect that calls a
      // function which *ever* calls setState, even asynchronously inside a
      // .then()/WebSocket handler -- which means it flags the standard,
      // React-docs-endorsed "fetch/connect data on mount" pattern used
      // throughout this codebase, not just genuine bugs. useRealtimeCollaboration.ts
      // already contains a deliberate refactor attempting to satisfy this rule
      // (splitting the state-setting call from the mount-effect call site) and
      // still trips it, confirming the rule can't be satisfied here without a
      // much larger architectural change. Downgraded to warn rather than left
      // as a hard build-breaking error.
      "react-hooks/set-state-in-effect": "warn",
      "react-hooks/purity": "warn",
    },
  },
]);

export default eslintConfig;

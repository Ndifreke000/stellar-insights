// ESLint 9+ requires flat config; @react-native/eslint-config only supports
// legacy .eslintrc format via `@react-native` (its main export) and flat
// config via `@react-native/eslint-config/flat`. This replaces the old
// .eslintrc.js (root/extends/parser/plugins were the legacy equivalent of
// the base array below; the two custom rules are carried over as-is).
const baseConfig = require('@react-native/eslint-config/flat');
const typescriptPlugin = require('@typescript-eslint/eslint-plugin');
const jestPlugin = require('eslint-plugin-jest');

module.exports = [
  ...baseConfig,
  {
    // eslint-plugin-ft-flow@2.0.3 (nested inside @react-native/eslint-config,
    // enabled by the base config for **/*.js) crashes under ESLint 9's rule
    // API (context.getAllComments is not a function -- an ESLint-8-only
    // method the plugin's isNoFlowFile check still calls). This codebase is
    // TypeScript-only, not Flow, so these rules have nothing to check here
    // anyway; disabling them avoids depending on a fix from a third-party
    // plugin that hasn't caught up to ESLint 9 yet.
    files: ['**/*.js'],
    rules: {
      'ft-flow/define-flow-type': 'off',
      'ft-flow/use-flow-type': 'off',
    },
  },
  {
    // jest.setup.js runs under Jest (it's the setupFilesAfterEach entry) but
    // doesn't match the base config's test-file glob
    // (**/*.{spec,test}.{js,ts,tsx} or **/__{mocks,tests}__/**), so it never
    // got the jest global (jest.mock, jest.fn, ...) registered -- every
    // reference to `jest` in this file was flagged as undefined.
    files: ['jest.setup.js'],
    languageOptions: {
      globals: {
        ...jestPlugin.environments.globals.globals,
      },
    },
  },
  {
    // Flat config resolves rule names to a plugin registered on this same
    // config object (not just anywhere earlier in the array), so the
    // plugin has to be re-registered here even though baseConfig already
    // uses it internally.
    plugins: {
      '@typescript-eslint': typescriptPlugin,
    },
    rules: {
      'react-native/no-inline-styles': 'warn',
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    },
  },
];

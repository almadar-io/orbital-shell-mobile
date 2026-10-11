const almadarPlugin = require("../almadar-eslint");
const tsParser = require("@typescript-eslint/parser");

module.exports = [
  // TypeScript + JSX parser
  {
    files: ["**/*.ts", "**/*.tsx"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaFeatures: { jsx: true },
        ecmaVersion: "latest",
        sourceType: "module",
      },
    },
  },

  // General rules for all TS files
  {
    files: ["**/*.ts", "**/*.tsx"],
    plugins: {
      almadar: almadarPlugin,
    },
    rules: {
      "almadar/no-as-any": "error",
      "almadar/no-import-generated": "error",
    },
  },

  // Ignores
  {
    ignores: [
      "**/node_modules/**",
      "**/dist/**",
      "packages/mobile/node_modules/**",
      "packages/server/node_modules/**",
      "packages/mobile/src/generated/**",
    ],
  },
];

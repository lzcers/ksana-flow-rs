import js from "@eslint/js";
import globals from "globals";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";
import tseslint from "typescript-eslint";
import { defineConfig, globalIgnores } from "eslint/config";

export default defineConfig([
    globalIgnores(["dist"]),
    {
        files: ["**/*.{ts,tsx}"],
        extends: [js.configs.recommended, tseslint.configs.recommended, reactHooks.configs.flat.recommended, reactRefresh.configs.vite],
        languageOptions: {
            ecmaVersion: 2020,
            globals: globals.browser,
        },
        rules: {
            // Disable strict any checking - allow any types in this codebase
            "@typescript-eslint/no-explicit-any": "off",
            // Disable react-hooks/set-state-in-effect - allow setState in useEffect
            "react-hooks/set-state-in-effect": "off",
            // Disable react-hooks/preserve-memoization - allow React Compiler to skip optimization
            "react-hooks/preserve-memoization": "off",
            // Disable unused vars warnings for variables starting with _
            "@typescript-eslint/no-unused-vars": ["error", { argsIgnorePattern: "^_", varsIgnorePattern: "^_" }],
            // Allow empty block statements with comments
            "no-empty": ["error", { allowEmptyCatch: true, allowEmptyLoopBody: true }],
        },
    },
]);

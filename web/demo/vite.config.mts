import { defineConfig, PluginOption } from "vite";

// There's some issue with ES modules and this dependency.
// Without this hack, there is an error 'monacoEditorPlugin is not a function'.
import _monacoEditorPlugin from "vite-plugin-monaco-editor";
const monacoEditorPlugin =
  (_monacoEditorPlugin as any).default || _monacoEditorPlugin;

export default defineConfig({
  plugins: [monacoEditorPlugin({ languageWorkers: ["editorWorkerService"] })],
  build: {
    // for top-level await
    target: "esnext",
  },
  // configure the dev server
  server: {
    fs: {
      // the wasm files are up one level
      allow: [".."],
    },
  },
});

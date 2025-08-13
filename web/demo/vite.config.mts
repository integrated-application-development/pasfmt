import { defineConfig } from "vite";
import path, { dirname, resolve } from "path";
import { mkdirSync, readFileSync, writeFileSync } from "fs";

// There's some issue with ES modules and this dependency.
// Without this hack, there is an error 'monacoEditorPlugin is not a function'.
import _monacoEditorPlugin from "vite-plugin-monaco-editor";
const monacoEditorPlugin =
  (_monacoEditorPlugin as any).default || _monacoEditorPlugin;

let keywordGenPlugin = {
  name: "generate-keywords-module",
  buildStart() {
    const keywords = readFileSync("../../misc/keywords.txt")
      .toString()
      .split("\n")
      .map((s) => s.trim());

    const content =
      "export const keywords = [\n" +
      keywords.map((kw) => `  "${kw}",\n`).join("") +
      "];";

    const filePath = resolve(__dirname, "src/generated/keywords.js");
    mkdirSync(dirname(filePath), { recursive: true });
    writeFileSync(filePath, content);
    console.log("generated:", filePath);
  },
};

export default defineConfig({
  plugins: [
    monacoEditorPlugin({
      languageWorkers: ["editorWorkerService"],
      // workaround from https://github.com/vdesjs/vite-plugin-monaco-editor/issues/44
      customDistPath(root, buildOutDir) {
        return path.join(root, buildOutDir, "monacoeditorwork");
      },
    }),
    keywordGenPlugin,
  ],
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
  base: "/pasfmt/",
});

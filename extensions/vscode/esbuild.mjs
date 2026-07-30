import * as esbuild from "esbuild";

const watch = process.argv.includes("--watch");
const extensionOptions = {
  entryPoints: ["src/extension.ts"],
  bundle: true,
  format: "cjs",
  platform: "node",
  target: "node18",
  outfile: "dist/extension.js",
  external: ["vscode"],
  sourcemap: true,
  logLevel: "info",
};
const webviewOptions = {
  entryPoints: ["src/webview/chat.ts"],
  bundle: true,
  format: "iife",
  platform: "browser",
  target: "es2022",
  outfile: "dist/chat.js",
  sourcemap: true,
  logLevel: "info",
};

if (watch) {
  const [extension, webview] = await Promise.all([esbuild.context(extensionOptions), esbuild.context(webviewOptions)]);
  await Promise.all([extension.watch(), webview.watch()]);
} else {
  await Promise.all([esbuild.build(extensionOptions), esbuild.build(webviewOptions)]);
}

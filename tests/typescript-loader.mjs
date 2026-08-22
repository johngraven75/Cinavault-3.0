import { readFile } from "node:fs/promises";
import { transform } from "esbuild";

export async function load(url, context, nextLoad) {
  if (url.endsWith(".ts") || url.endsWith(".tsx")) {
    const source = await readFile(new URL(url), "utf8");
    const result = await transform(source, {
      loader: url.endsWith(".tsx") ? "tsx" : "ts",
      format: "esm",
      target: "es2022",
      sourcemap: "inline",
    });
    return { format: "module", source: result.code, shortCircuit: true };
  }
  return nextLoad(url, context);
}

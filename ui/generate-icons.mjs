// Rasterise assets/img/kamosu-mark.svg into every PNG size a PWA install and a
// browser tab need (issue #80). Run via `just css` / `npm run icons`.
//
// The mark fills its square edge-to-edge, so one piece of art serves both the
// "any" and "maskable" purposes; the OS does the masking.
import { Resvg } from "@resvg/resvg-js";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { DEFAULT_STYLESHEET, token } from "./tokens.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const svg = readFileSync(path.join(root, "assets/img/kamosu-mark.svg"), "utf8");

const sizes = [
  ["icon-192.png", 192],
  ["icon-512.png", 512],
  ["apple-touch-icon.png", 180], // iOS home screen; no transparency allowed
  ["favicon-32.png", 32], // browser tab fallback for browsers without SVG favicons
];

// Optional first argument redirects output elsewhere, and an optional second
// names the stylesheet to read the colour from — both used by the freshness
// check, which regenerates into a temp directory instead of touching the tree.
const outDir = process.argv[2]
  ? path.resolve(process.cwd(), process.argv[2])
  : path.join(root, "assets/icons");
const stylesheet = process.argv[3]
  ? path.resolve(process.cwd(), process.argv[3])
  : DEFAULT_STYLESHEET;

// iOS composites the home-screen icon onto an opaque square, so it needs a
// background — read from the stylesheet, never typed here.
const accent = token("--color-accent", stylesheet);

for (const [name, size] of sizes) {
  const resvg = new Resvg(svg, {
    fitTo: { mode: "width", value: size },
    background: name === "apple-touch-icon.png" ? accent : undefined,
  });
  const png = resvg.render().asPng();
  writeFileSync(path.join(outDir, name), png);
  console.log(`wrote ${path.relative(root, path.join(outDir, name))} (${png.length} bytes)`);
}

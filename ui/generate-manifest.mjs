// The web manifest, generated (issue #37).
//
// It names the icons the binary serves at /assets/icons/* and carries two of
// Kamosu's colours. Those colours are read out of the generated stylesheet
// rather than typed here, so `assets/app.css` stays the single source of what
// Kamosu looks like (AGENTS.md) and the manifest cannot fall behind it.
import { writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { DEFAULT_STYLESHEET, token } from "./tokens.mjs";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

// Optional arguments redirect the output and name the stylesheet to read —
// used by the freshness check, which regenerates into a temp directory.
const outFile = process.argv[2]
  ? path.resolve(process.cwd(), process.argv[2])
  : path.join(root, "ui/static/manifest.webmanifest");
const stylesheet = process.argv[3]
  ? path.resolve(process.cwd(), process.argv[3])
  : DEFAULT_STYLESHEET;

const ground = token("--color-ground", stylesheet);

const manifest = {
  name: "Kamosu",
  short_name: "Kamosu",
  description: "A self-hosted cookbook where people and software agents are equal users.",
  start_url: "/",
  scope: "/",
  display: "standalone",
  background_color: ground,
  theme_color: ground,
  icons: [
    { src: "/assets/icons/icon-192.png", sizes: "192x192", type: "image/png" },
    { src: "/assets/icons/icon-512.png", sizes: "512x512", type: "image/png" },
    // The mark fills its square edge-to-edge, so one piece of art serves both
    // purposes; the OS does the masking.
    { src: "/assets/icons/icon-512.png", sizes: "512x512", type: "image/png", purpose: "maskable" },
  ],
};

writeFileSync(outFile, JSON.stringify(manifest, null, "\t") + "\n");
console.log(`wrote ${path.relative(root, outFile)}`);

import { rm, cp, mkdir } from "fs/promises";
import { join } from "path";

const dist = join(import.meta.dir, "dist");

// Clean previous output
await rm(dist, { recursive: true, force: true });
await mkdir(dist, { recursive: true });

// Bundle the React app
const result = await Bun.build({
	entrypoints: ["src/mainview/main.tsx"],
	outdir: dist,
	minify: true,
	target: "browser",
	naming: {
		entry: "assets/[name]-[hash].[ext]",
		asset: "assets/[name]-[hash].[ext]",
	},
});

if (!result.success) {
	console.error("Build failed:");
	for (const log of result.logs) {
		console.error(log);
	}
	process.exit(1);
}

// Find output filenames for the HTML template
const jsFile = result.outputs.find((o) => o.path.endsWith(".js"));
const cssFile = result.outputs.find((o) => o.path.endsWith(".css"));

if (!jsFile) {
	console.error("No JS output produced");
	process.exit(1);
}

const jsName = "assets/" + jsFile.path.split("/assets/")[1];
const cssTag = cssFile
	? `<link rel="stylesheet" href="/${
			"assets/" + cssFile.path.split("/assets/")[1]
		}">`
	: "";

// Generate index.html from template
const html = `<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>Nomen</title>
    ${cssTag}
  </head>
  <body>
    <div id="root"></div>
    <script type="module" src="/${jsName}"></script>
  </body>
</html>`;

await Bun.write(join(dist, "index.html"), html);

console.log("✓ built to dist/");
for (const output of result.outputs) {
	const rel = output.path.replace(dist + "/", "");
	const kb = (output.size / 1024).toFixed(1);
	console.log(`  ${rel}  ${kb} kB`);
}

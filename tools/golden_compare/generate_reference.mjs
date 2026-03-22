#!/usr/bin/env node
/**
 * Generate KaTeX reference PNGs for golden test comparison.
 * Reads test_cases.txt, renders each formula with KaTeX in a headless browser,
 * and saves screenshots to the fixtures directory.
 *
 * Usage:
 *   node generate_reference.mjs [test_cases.txt] [fixtures_dir] [--mhchem]
 *
 * --mhchem: use 40px font (for tests/golden/test_case_ce.txt → fixtures_ce).
 * mhchem (\\ce, \\pu, …) is loaded after KaTeX via Puppeteer addScriptTag so file://
 * reference runs always register macros; do not rely on a second <script src="contrib/…"> alone.
 * KaTeX dist is resolved from tools/golden_compare/node_modules or tools/lexer_compare/node_modules.
 */
import { readFileSync, writeFileSync, unlinkSync, mkdirSync, existsSync } from 'fs';
import { dirname, join } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';
import puppeteer from 'puppeteer';

const __dirname = dirname(fileURLToPath(import.meta.url));

function resolveKatexDist() {
    const candidates = [
        join(__dirname, 'node_modules', 'katex', 'dist'),
        join(__dirname, '..', 'lexer_compare', 'node_modules', 'katex', 'dist'),
    ];
    for (const c of candidates) {
        const katexJs = join(c, 'katex.min.js');
        const mhchemJs = join(c, 'contrib', 'mhchem.min.js');
        if (existsSync(katexJs) && existsSync(mhchemJs)) {
            return c;
        }
    }
    throw new Error(
        'KaTeX dist not found or missing contrib/mhchem.min.js (required for \\ce and \\pu). ' +
            'Run: (cd tools/golden_compare && npm install) or npm install under tools/lexer_compare'
    );
}

async function main() {
    const rawArgs = process.argv.slice(2);
    const withMhchem = rawArgs.includes('--mhchem');
    const args = rawArgs.filter((a) => a !== '--mhchem');
    const testCasesPath =
        args[0] || join(__dirname, '..', '..', 'tests', 'golden', 'test_cases.txt');
    const outputDir =
        args[1] || join(__dirname, '..', '..', 'tests', 'golden', 'fixtures');

    const KATEX_DIST = resolveKatexDist();
    const fontPx = withMhchem ? 40 : 20;

    if (!existsSync(outputDir)) {
        mkdirSync(outputDir, { recursive: true });
    }

    const lines = readFileSync(testCasesPath, 'utf8')
        .split('\n')
        .filter(l => l.trim() && !l.trim().startsWith('#'));

    console.log(
        `Generating ${lines.length} reference PNGs (KaTeX + mhchem, ${fontPx}px)...`
    );

    // Write temp HTML in KaTeX dist dir so relative font paths resolve correctly
    const tempHtml = join(KATEX_DIST, '_golden_render.html');
    const html = `<!DOCTYPE html>
<html>
<head>
<link rel="stylesheet" href="katex.min.css">
<style>
body { margin: 0; padding: 0; background: white; }
#formula {
    display: inline-block;
    padding: 10px;
    font-size: ${fontPx}px;
}
</style>
<script src="katex.min.js"></script>
</head>
<body>
<div id="formula"></div>
</body>
</html>`;
    writeFileSync(tempHtml, html);

    const browser = await puppeteer.launch({
        headless: true,
        args: ['--no-sandbox', '--disable-setuid-sandbox', '--allow-file-access-from-files'],
    });

    const page = await browser.newPage();
    await page.setViewport({ width: 800, height: 600, deviceScaleFactor: 2 });

    // Navigate to file URL — CSS relative paths (fonts/...) resolve from KaTeX dist dir
    await page.goto(pathToFileURL(tempHtml).href, { waitUntil: 'networkidle0' });

    // Load mhchem after KaTeX (defines \\ce, \\pu, …). Using addScriptTag avoids file:// edge
    // cases where a relative contrib/ script may not run before the first render.
    await page.addScriptTag({
        path: join(KATEX_DIST, 'contrib', 'mhchem.min.js'),
    });

    let ok = 0;
    let errors = 0;
    let fontsChecked = false;
    for (let i = 0; i < lines.length; i++) {
        const expr = lines[i].trim();
        const idx = String(i + 1).padStart(4, '0');

        try {
            await page.evaluate(async (expr) => {
                const el = document.getElementById('formula');
                el.innerHTML = '';
                // displayMode: true + outer `$...$` breaks KaTeX; strip one pair for display.
                let toRender = expr;
                let displayMode = true;
                const outer = toRender.match(/^\$(.*)\$$/s);
                if (outer) {
                    toRender = outer[1];
                }
                katex.render(toRender, el, {
                    displayMode,
                    throwOnError: false,
                    trust: true,
                });
                await document.fonts.ready;
            }, expr);

            await page.waitForSelector('#formula .katex', { timeout: 2000 });

            // Verify fonts loaded after first render
            if (!fontsChecked) {
                const fontsLoaded = await page.evaluate(async () => {
                    await document.fonts.ready;
                    const loaded = [];
                    for (const font of document.fonts) {
                        if (font.status === 'loaded') loaded.push(font.family);
                    }
                    return [...new Set(loaded)];
                });
                console.log(`KaTeX fonts loaded: ${fontsLoaded.length} families`);
                if (fontsLoaded.length > 0) {
                    console.log(`  ${fontsLoaded.join(', ')}`);
                } else {
                    console.error('WARNING: No KaTeX fonts loaded! References use system fallback fonts.');
                }
                fontsChecked = true;
            }

            const element = await page.$('#formula');
            const box = await element.boundingBox();
            if (box && box.width > 0 && box.height > 0) {
                await element.screenshot({
                    path: join(outputDir, `${idx}.png`),
                    omitBackground: false,
                });
                ok++;
                if ((i + 1) % 50 === 0) {
                    console.log(`  ${i + 1}/${lines.length} done...`);
                }
            } else {
                console.error(`SKIP ${idx}: empty bounding box for "${expr}"`);
                errors++;
            }
        } catch (err) {
            console.error(`ERR  ${idx}: ${expr} — ${err.message}`);
            errors++;
        }
    }

    await browser.close();

    // Clean up temp file
    try { unlinkSync(tempHtml); } catch (_) {}

    console.log(`\nDone: ${ok} OK, ${errors} errors out of ${lines.length} formulas`);
    console.log(`Reference PNGs saved to ${outputDir}/`);
}

main().catch(err => {
    console.error(err);
    process.exit(1);
});

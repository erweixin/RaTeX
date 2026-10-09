#!/usr/bin/env node
/**
 * Generate KaTeX reference PNGs for golden test comparison.
 * Reads test_cases.txt, renders each formula with KaTeX in a headless browser,
 * and saves screenshots to the fixtures directory.
 *
 * Usage:
 *   node generate_reference.mjs [test_cases.txt] [fixtures_dir] [--mhchem] [--centernot]
 *
 * --mhchem: use 40px font (for tests/golden/test_case_ce.txt → fixtures_ce).
 * mhchem (\\ce, \\pu, …) is loaded after KaTeX via Puppeteer addScriptTag so file://
 * reference runs always register macros; do not rely on a second <script src="contrib/…"> alone.
 * KaTeX dist is resolved from tools/golden_compare/node_modules or tools/lexer_compare/node_modules.
 *
 * The reference implementation is intentionally fixed at KaTeX 0.16.45.
 * Upgrade it only in a dedicated reference-baseline change.
 */
import {
    readFileSync,
    writeFileSync,
    unlinkSync,
    mkdirSync,
    existsSync,
    readdirSync,
} from 'fs';
import { createHash } from 'crypto';
import { dirname, join, relative, resolve } from 'path';
import { fileURLToPath, pathToFileURL } from 'url';
import puppeteer from 'puppeteer';
import { PUPPETEER_REVISIONS } from 'puppeteer-core/internal/revisions.js';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = join(__dirname, '..', '..');
const EXPECTED_KATEX_VERSION = '0.16.45';
const VIEWPORT_DPR = 2;

function sha256File(path) {
    return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function fontHashes(fontDir) {
    return Object.fromEntries(
        readdirSync(fontDir)
            .filter((name) => /\.(?:woff2?|ttf)$/i.test(name))
            .sort()
            .map((name) => [name, sha256File(join(fontDir, name))])
    );
}

/** Read PNG width/height from IHDR (no extra deps). */
function readPngSize(absPath) {
    const buf = readFileSync(absPath);
    if (buf.length < 24) {
        throw new Error(`PNG too small: ${absPath}`);
    }
    if (buf[0] !== 0x89 || buf[1] !== 0x50 || buf[2] !== 0x4e || buf[3] !== 0x47) {
        throw new Error(`Not a PNG: ${absPath}`);
    }
    return {
        width: buf.readUInt32BE(16),
        height: buf.readUInt32BE(20),
    };
}

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
    const withCenternot = rawArgs.includes('--centernot');
    // KaTeX 0.16.45 has no centernot. This independent box construction
    // implements centernot.sty v1.4: shift zero-advance \not by (W - E)/2,
    // where W is the argument width and E the equals-sign width. The clap
    // shifts by -(W + E)/2, then the first phantom advances by W.
    const referenceMacros = withCenternot ? {
        '\\centernot': String.raw`\mathrel{\mathclap{{\phantom{#1}}{\not}{\phantom{=}}}{#1}}`,
    } : {};
    const manifestArg = rawArgs.indexOf('--manifest-out');
    const manifestOutArg = manifestArg >= 0 ? rawArgs[manifestArg + 1] : null;
    if (manifestArg >= 0 && !manifestOutArg) {
        throw new Error('--manifest-out requires a path');
    }
    const args = rawArgs.filter(
        (arg, index) =>
            arg !== '--mhchem' && arg !== '--centernot' &&
            (manifestArg < 0 ||
                (index !== manifestArg && index !== manifestArg + 1))
    );
    const testCasesPath =
        args[0] || join(__dirname, '..', '..', 'tests', 'golden', 'test_cases.txt');
    const outputDir =
        args[1] || join(__dirname, '..', '..', 'tests', 'golden', 'fixtures');
    // When set, numbered-env fixtures use the same total canvas width as RaTeX PNGs
    // (tests/golden/output/NNNN.png) so golden ink comparison is not stretched by a
    // 700px-wide KaTeX container. Generate output first: scripts/update_golden_output.sh
    const alignOutputDir = join(__dirname, '..', '..', 'tests', 'golden', 'output');

    const KATEX_DIST = resolveKatexDist();
    const katexPackage = JSON.parse(
        readFileSync(join(KATEX_DIST, '..', 'package.json'), 'utf8')
    );
    if (katexPackage.version !== EXPECTED_KATEX_VERSION) {
        throw new Error(
            `KaTeX ${EXPECTED_KATEX_VERSION} is required, but ${katexPackage.version} is installed. ` +
                'Run npm ci in tools/golden_compare.'
        );
    }
    const puppeteerPackage = JSON.parse(
        readFileSync(join(__dirname, 'node_modules', 'puppeteer', 'package.json'), 'utf8')
    );
    const fontPx = withMhchem ? 40 : 20;
    const manifestOut = manifestOutArg || join(outputDir, 'reference-manifest.json');

    if (!existsSync(outputDir)) {
        mkdirSync(outputDir, { recursive: true });
    }
    if (existsSync(manifestOut)) {
        unlinkSync(manifestOut);
    }
    for (const name of readdirSync(outputDir)) {
        if (/^\d{4,}\.png$/.test(name)) {
            unlinkSync(join(outputDir, name));
        }
    }

    const lines = readFileSync(testCasesPath, 'utf8')
        .split('\n')
        .filter(l => {
            const formula = l.trim();
            return formula && !formula.startsWith('#') && !formula.startsWith('%');
        });

    console.log(
        `Generating ${lines.length} reference PNGs (KaTeX + mhchem, ${fontPx}px)...`
    );

    // Write temp HTML in KaTeX dist dir so relative font paths resolve correctly.
    //
    // Default rendering keeps the historical inline-block + 10px-padding layout
    // so screenshots of fixtures without numbered environments are byte-for-byte
    // identical to what the suite produced before the tag-overlap fix landed.
    //
    // For \\begin{align} / \\begin{gather} / \\tag{} we detect the .tag element
    // afterwards and re-render the same expression in a wide block container
    // (set on #formula via inline style) so the absolutely-positioned tag has
    // room at the right and doesn't overlap the equation. The screenshot for
    // those cases clips to the union of .base + .tag bounds.
    const STAGE_WIDTH = 720;
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
#formula.tagged {
    display: block;
    width: ${STAGE_WIDTH - 20}px;
    padding: 10px;
    position: relative;
}
/* KaTeX 0.16.x with fleqn:true puts padding-left:2em on
 * .katex-display.fleqn > .katex. Zero it so the equation .base sits at x=0
 * while the absolutely-positioned .tag still anchors to right:0. Margin reset
 * keeps vertical bounds tight. */
#formula.tagged .katex-display { margin: 0; }
#formula.tagged .katex-display.fleqn > .katex { padding-left: 0; padding-right: 0; }
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
    const browserVersion = await browser.version();

    const page = await browser.newPage();
    await page.setViewport({
        width: STAGE_WIDTH + 80,
        height: 1024,
        deviceScaleFactor: VIEWPORT_DPR,
    });

    // Navigate to file URL — CSS relative paths (fonts/...) resolve from KaTeX dist dir
    await page.goto(pathToFileURL(tempHtml).href, { waitUntil: 'networkidle0' });

    // Load mhchem after KaTeX (defines \\ce, \\pu, …). Using addScriptTag avoids file:// edge
    // cases where a relative contrib/ script may not run before the first render.
    await page.addScriptTag({
        path: join(KATEX_DIST, 'contrib', 'mhchem.min.js'),
    });

    await page.evaluate((macros) => {
        for (const [name, expansion] of Object.entries(macros)) {
            katex.__defineMacro(name, expansion);
        }
    }, referenceMacros);

    let ok = 0;
    let errors = 0;
    let fontsChecked = false;
    const records = [];
    for (let i = 0; i < lines.length; i++) {
        const expr = lines[i].trim();
        const idx = String(i + 1).padStart(4, '0');

        try {
            // Pass 1: render in the default inline-block container.
            const hasTag = await page.evaluate(async (expr) => {
                const el = document.getElementById('formula');
                el.className = '';
                el.style.width = '';
                el.innerHTML = '';
                let toRender = expr;
                const outer = toRender.match(/^\$(.*)\$$/s);
                if (outer) toRender = outer[1];
                katex.render(toRender, el, {
                    displayMode: true,
                    throwOnError: true,
                    trust: true,
                    strict: false,
                });
                await document.fonts.ready;
                return el.querySelector('.tag') !== null;
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

            if (!hasTag) {
                // Original behavior: screenshot the inline-block #formula box.
                // This preserves the historical fixture geometry for every
                // formula that does not produce a KaTeX `.tag` element.
                const element = await page.$('#formula');
                const box = await element.boundingBox();
                if (box && box.width > 0 && box.height > 0) {
                    await element.screenshot({
                        path: join(outputDir, `${idx}.png`),
                        omitBackground: false,
                    });
                    ok++;
                    records.push({
                        index: i + 1,
                        formula: expr,
                        status: 'rendered',
                        png: `${idx}.png`,
                        sha256: sha256File(join(outputDir, `${idx}.png`)),
                    });
                } else {
                    console.error(`SKIP ${idx}: empty bounding box for "${expr}"`);
                    errors++;
                    records.push({
                        index: i + 1,
                        formula: expr,
                        status: 'render_error',
                        reason: 'empty bounding box',
                    });
                }
            } else {
                // Pass 2: block container, equation left-aligned (fleqn:true) so the
                // absolutely-positioned `.tag` (right:0) sits in the right margin
                // instead of overlapping a centered equation body in narrow widths.
                //
                // Width target = RaTeX canvas width when tests/golden/output/<idx>.png
                // exists, but we expand if the natural equation+tag is wider than that.

                // Pre-measure natural width of equation + tag at fleqn (need a roomy parent).
                const naturalNeed = await page.evaluate((expr) => {
                    const el = document.getElementById('formula');
                    el.className = 'tagged';
                    el.style.width = '2000px';
                    el.innerHTML = '';
                    let toRender = expr;
                    const outer = toRender.match(/^\$(.*)\$$/s);
                    if (outer) toRender = outer[1];
                    katex.render(toRender, el, {
                        displayMode: true,
                        throwOnError: true,
                        trust: true,
                        strict: false,
                        fleqn: true,
                    });
                    let maxBaseR = 0;
                    let maxTagW = 0;
                    const elRect = el.getBoundingClientRect();
                    for (const b of el.querySelectorAll('.base')) {
                        const r = b.getBoundingClientRect();
                        if (r.right - elRect.left > maxBaseR) {
                            maxBaseR = r.right - elRect.left;
                        }
                    }
                    for (const t of el.querySelectorAll('.tag')) {
                        const r = t.getBoundingClientRect();
                        if (r.width > maxTagW) maxTagW = r.width;
                    }
                    // 16 CSS px ≈ 1em-ish min gap so .tag doesn't sit on the equation.
                    return Math.ceil(maxBaseR + 16 + maxTagW);
                }, expr);

                let contentWidthCss = STAGE_WIDTH - 20;
                const alignPng = join(alignOutputDir, `${idx}.png`);
                if (existsSync(alignPng)) {
                    try {
                        const { width: wOut } = readPngSize(alignPng);
                        contentWidthCss = wOut / VIEWPORT_DPR - 20;
                    } catch (e) {
                        console.error(`WARN ${idx}: could not read align PNG: ${e.message}`);
                    }
                }
                contentWidthCss = Math.max(48, contentWidthCss, naturalNeed);

                await page.evaluate(
                    async (expr, contentW) => {
                        const el = document.getElementById('formula');
                        el.className = 'tagged';
                        el.style.width = `${contentW}px`;
                        el.innerHTML = '';
                        let toRender = expr;
                        const outer = toRender.match(/^\$(.*)\$$/s);
                        if (outer) toRender = outer[1];
                        katex.render(toRender, el, {
                            displayMode: true,
                            throwOnError: true,
                            trust: true,
                            strict: false,
                            fleqn: true,
                        });
                        await document.fonts.ready;
                    },
                    expr,
                    contentWidthCss
                );

                const element = await page.$('#formula');
                const box = await element.boundingBox();
                if (box && box.width > 0 && box.height > 0) {
                    await element.screenshot({
                        path: join(outputDir, `${idx}.png`),
                        omitBackground: false,
                    });
                    ok++;
                    records.push({
                        index: i + 1,
                        formula: expr,
                        status: 'rendered',
                        png: `${idx}.png`,
                        sha256: sha256File(join(outputDir, `${idx}.png`)),
                    });
                } else {
                    console.error(`SKIP ${idx}: empty bounding box for "${expr}"`);
                    errors++;
                    records.push({
                        index: i + 1,
                        formula: expr,
                        status: 'render_error',
                        reason: 'empty bounding box',
                    });
                }
            }

            if ((i + 1) % 50 === 0) {
                console.log(`  ${i + 1}/${lines.length} done...`);
            }
        } catch (err) {
            console.error(`ERR  ${idx}: ${expr} — ${err.message}`);
            errors++;
            records.push({
                index: i + 1,
                formula: expr,
                status:
                    err?.name === 'ParseError' || /parse error/i.test(err?.message || '')
                        ? 'parse_error'
                        : 'render_error',
                reason: err?.message || String(err),
            });
        }
    }

    await browser.close();

    // Clean up temp file
    try { unlinkSync(tempHtml); } catch (_) {}

    const manifest = {
        manifest_version: 1,
        kind: 'katex-reference',
        generated_at: new Date().toISOString(),
        test_cases: relative(REPO_ROOT, resolve(testCasesPath)).replaceAll('\\', '/'),
        test_cases_sha256: sha256File(testCasesPath),
        case_count: lines.length,
        katex_version: katexPackage.version,
        puppeteer_version: puppeteerPackage.version,
        chromium_revision: PUPPETEER_REVISIONS.chrome,
        chromium_version: browserVersion,
        dpr: VIEWPORT_DPR,
        font_px: fontPx,
        font_file_hashes: fontHashes(join(KATEX_DIST, 'fonts')),
        ...(withCenternot ? { reference_macros: referenceMacros } : {}),
        cases: records,
    };
    writeFileSync(manifestOut, JSON.stringify(manifest, null, 2) + '\n');

    console.log(`\nDone: ${ok} OK, ${errors} errors out of ${lines.length} formulas`);
    console.log(`Reference PNGs saved to ${outputDir}/`);
    console.log(`Reference manifest saved to ${manifestOut}`);
    if (errors > 0) {
        process.exitCode = 1;
    }
}

main().catch(err => {
    console.error(err);
    process.exit(1);
});

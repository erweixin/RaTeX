import assert from "node:assert/strict";
import test from "node:test";
import sharp from "sharp";
import sharpService from "astro/assets/services/sharp";

test("Astro resizes an SVG through sharp and emits a valid PNG", async () => {
  const input = Buffer.from(
    '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="16"><rect width="24" height="16" fill="#336699"/></svg>'
  );
  const result = await sharpService.transform(
    input,
    { width: 12, format: "png" },
    { service: { config: { limitInputPixels: 1024 } } }
  );
  const metadata = await sharp(result.data).metadata();

  assert.equal(result.format, "png");
  assert.equal(metadata.format, "png");
  assert.equal(metadata.width, 12);
  assert.equal(metadata.height, 8);
});

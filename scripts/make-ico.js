// SVG から PNG 入りの .ico を作る（16〜256px）。
//
// sharp が要る。このリポジトリには node の依存が無いので、sharp のある node_modules を NODE_PATH で指す:
//   NODE_PATH=path/to/node_modules node scripts/make-ico.js assets/icon-error.svg assets/icon-error.ico
const fs = require("fs");
const sharp = require("sharp");

const [svgPath, icoPath] = process.argv.slice(2);
if (!svgPath || !icoPath) {
  console.error("使い方: node scripts/make-ico.js <in.svg> <out.ico>");
  process.exit(1);
}
const sizes = [16, 20, 24, 32, 40, 48, 64, 128, 256];

(async () => {
  const pngs = [];
  for (const s of sizes) {
    pngs.push({ s, buf: await sharp(svgPath).resize(s, s).png().toBuffer() });
  }
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0); // reserved
  header.writeUInt16LE(1, 2); // 1 = icon
  header.writeUInt16LE(pngs.length, 4);
  const dir = Buffer.alloc(16 * pngs.length);
  let offset = 6 + dir.length;
  pngs.forEach(({ s, buf }, i) => {
    const e = i * 16;
    dir.writeUInt8(s === 256 ? 0 : s, e); // 256 は 0 で表す
    dir.writeUInt8(s === 256 ? 0 : s, e + 1);
    dir.writeUInt8(0, e + 2); // palette
    dir.writeUInt8(0, e + 3); // reserved
    dir.writeUInt16LE(1, e + 4); // planes
    dir.writeUInt16LE(32, e + 6); // bpp
    dir.writeUInt32LE(buf.length, e + 8);
    dir.writeUInt32LE(offset, e + 12);
    offset += buf.length;
  });
  fs.writeFileSync(icoPath, Buffer.concat([header, dir, ...pngs.map((p) => p.buf)]));
  console.log(`${icoPath}: ${sizes.join("/")} px, ${offset} bytes`);
})().catch((e) => {
  console.error(e.message);
  process.exit(1);
});

// Minimal static server for browser checks: node serve.mjs <port> <urlPrefix>=<dir> [...]
// e.g. node serve.mjs 8765 /projects/autocolor/=dist  (longest matching prefix wins)
import http from 'node:http';
import { createReadStream, statSync } from 'node:fs';
import { join, normalize, extname, resolve } from 'node:path';

const [port, ...mounts] = process.argv.slice(2);
const table = mounts.map((m) => m.split('=')).map(([p, d]) => [p, resolve(d)]).sort((a, b) => b[0].length - a[0].length);
const TYPES = { '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.mjs': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm', '.json': 'application/json', '.svg': 'image/svg+xml', '.png': 'image/png', '.jpg': 'image/jpeg', '.webp': 'image/webp', '.ico': 'image/x-icon', '.woff2': 'font/woff2', '.glb': 'model/gltf-binary', '.hdr': 'application/octet-stream' };

http
  .createServer((req, res) => {
    const url = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    const hit = table.find(([p]) => url.startsWith(p));
    if (!hit) return res.writeHead(404).end('not found');
    let file = normalize(join(hit[1], url.slice(hit[0].length)));
    if (!file.startsWith(hit[1])) return res.writeHead(403).end();
    try {
      if (statSync(file).isDirectory()) file = join(file, 'index.html');
      statSync(file);
    } catch {
      return res.writeHead(404).end('not found');
    }
    res.writeHead(200, { 'content-type': TYPES[extname(file)] ?? 'application/octet-stream', 'cache-control': 'no-store' });
    createReadStream(file).pipe(res);
  })
  .listen(Number(port), '127.0.0.1', () => console.log(`serving on http://127.0.0.1:${port}`));

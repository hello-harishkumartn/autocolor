import test from 'node:test';
import assert from 'node:assert/strict';
import { crc32 as nodeCrc32 } from 'node:zlib';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createZip, crc32 } from '../../app/zip.js';

const bytes = (s) => new TextEncoder().encode(s);

test('crc32 matches zlib', () => {
  for (const s of ['', 'a', 'hello world', 'x'.repeat(10000)]) assert.equal(crc32(bytes(s)), nodeCrc32(Buffer.from(s)) >>> 0);
});

test('a store-only zip that unzip accepts, with UTF-8 names and the given date', async () => {
  const files = [
    { name: 'one-autocolor.jpg', data: bytes('first file') },
    { name: 'café-autocolor.jpg', data: new Uint8Array(70000).map((_, i) => i % 251) },
    { name: 'empty-autocolor.jpg', data: new Uint8Array(0) },
  ];
  const blob = createZip(files, new Date(2026, 10, 15, 13, 45, 10));
  assert.equal(blob.type, 'application/zip');
  const dir = mkdtempSync(join(tmpdir(), 'zip-'));
  const path = join(dir, 'out.zip');
  writeFileSync(path, Buffer.from(await blob.arrayBuffer()));
  const test = execFileSync('unzip', ['-t', path]).toString();
  assert.match(test, /No errors detected/);
  const list = execFileSync('unzip', ['-l', path]).toString();
  assert.match(list, /2026-11-15 13:45|11-15-2026 13:45/);
  // names and contents as an independent reader (Python's zipfile) sees them
  const read = JSON.parse(
    execFileSync('python3', ['-I', '-c', 'import json,sys,zipfile; z=zipfile.ZipFile(sys.argv[1]); print(json.dumps([[i.filename, z.read(i).hex()] for i in z.infolist()]))', path]).toString(),
  );
  assert.deepEqual(read.map((r) => r[0]), files.map((f) => f.name));
  read.forEach(([, hex], i) => assert.equal(hex, Buffer.from(files[i].data).toString('hex'), files[i].name));
  // general purpose flag bit 11 (UTF-8) on every local header
  const buf = readFileSync(path);
  assert.equal(buf.readUInt32LE(0), 0x04034b50);
  assert.equal(buf.readUInt16LE(6) & 0x0800, 0x0800);
  assert.equal(buf.readUInt16LE(8), 0, 'stored, not deflated');
});

test('entries can be Blobs with a known CRC and size (kept out of the JS heap)', async () => {
  const data = new Uint8Array(5000).map((_, i) => (i * 7) % 256);
  const blob = createZip([{ name: 'a-autocolor.jpg', blob: new Blob([data]), crc: crc32(data), size: data.length }]);
  const dir = mkdtempSync(join(tmpdir(), 'zip-'));
  const path = join(dir, 'b.zip');
  writeFileSync(path, Buffer.from(await blob.arrayBuffer()));
  assert.match(execFileSync('unzip', ['-t', path]).toString(), /No errors detected/);
  assert.deepEqual(new Uint8Array(execFileSync('unzip', ['-p', path, 'a-autocolor.jpg'])), data);
});

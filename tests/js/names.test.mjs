import test from 'node:test';
import assert from 'node:assert/strict';
import { outputName, uniqueNames, classify } from '../../app/names.js';

test('output names keep the base name and add -autocolor.jpg', () => {
  assert.equal(outputName('IMG_1.JPG'), 'IMG_1-autocolor.jpg');
  assert.equal(outputName('photo.final.png'), 'photo.final-autocolor.jpg');
  assert.equal(outputName('noext'), 'noext-autocolor.jpg');
  assert.equal(outputName('.hidden'), '.hidden-autocolor.jpg');
  assert.equal(outputName('folder/sub/pic.webp'), 'pic-autocolor.jpg');
  assert.equal(outputName(''), 'photo-autocolor.jpg');
});

test('names inside one zip are made unique', () => {
  assert.deepEqual(uniqueNames(['a-autocolor.jpg', 'b-autocolor.jpg', 'a-autocolor.jpg', 'A-autocolor.jpg']), [
    'a-autocolor.jpg',
    'b-autocolor.jpg',
    'a-autocolor (2).jpg',
    'A-autocolor (3).jpg',
  ]);
});

test('files are classified by type and extension', () => {
  assert.equal(classify('x.jpg', 'image/jpeg'), 'ok');
  assert.equal(classify('x.PNG', ''), 'ok');
  assert.equal(classify('x.webp', 'image/webp'), 'ok');
  assert.equal(classify('x.heic', ''), 'heic');
  assert.equal(classify('x.bin', 'image/heif'), 'heic');
  assert.equal(classify('x.CR3', ''), 'raw');
  assert.equal(classify('x.dng', 'image/x-adobe-dng'), 'raw');
  assert.equal(classify('notes.txt', 'text/plain'), 'unsupported');
  assert.equal(classify('x.gif', 'image/gif'), 'unsupported');
});

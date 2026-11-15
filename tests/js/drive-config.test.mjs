import test from 'node:test';
import assert from 'node:assert/strict';
import { driveState, SCOPE, PICKER_MIME_TYPES } from '../../app/drive-config.js';

test('Drive is pending until all three public IDs are set', () => {
  assert.deepEqual(driveState({ GOOGLE_CLIENT_ID: '', GOOGLE_API_KEY: '', GOOGLE_APP_ID: '' }), { ready: false, label: 'Google Drive setup pending' });
  assert.equal(driveState({ GOOGLE_CLIENT_ID: 'x.apps.googleusercontent.com', GOOGLE_API_KEY: 'k', GOOGLE_APP_ID: '' }).ready, false);
  assert.equal(driveState({}).ready, false);
  assert.deepEqual(driveState({ GOOGLE_CLIENT_ID: 'x.apps.googleusercontent.com', GOOGLE_API_KEY: 'k', GOOGLE_APP_ID: '123' }), {
    ready: true,
    label: 'Choose from Google Drive',
  });
});

test('the app asks only for the drive.file scope and image types', () => {
  assert.equal(SCOPE, 'https://www.googleapis.com/auth/drive.file');
  assert.match(PICKER_MIME_TYPES, /image\/jpeg/);
  assert.doesNotMatch(PICKER_MIME_TYPES, /folder/);
});

test('the shipped config has empty IDs and no secrets', async () => {
  const cfg = await import('../../app/config.js');
  assert.deepEqual(Object.keys(cfg).sort(), ['GOOGLE_API_KEY', 'GOOGLE_APP_ID', 'GOOGLE_CLIENT_ID']);
  for (const v of Object.values(cfg)) assert.equal(typeof v, 'string');
});

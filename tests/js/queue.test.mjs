import test from 'node:test';
import assert from 'node:assert/strict';
import { createQueue } from '../../app/queue.js';

const tick = (ms) => new Promise((r) => setTimeout(r, ms));

test('jobs run one at a time in order with progress', async () => {
  let running = 0;
  let maxRunning = 0;
  const done = [];
  const progress = [];
  const q = createQueue(async (job) => {
    running++;
    maxRunning = Math.max(maxRunning, running);
    await tick(5);
    running--;
    if (job === 'bad') throw new Error('broken photo');
    return job.toUpperCase();
  }, {
    onProgress: (p) => progress.push(`${p.done}/${p.total}`),
    onResult: (job, result, error) => done.push(error ? `${job}:${error.message}` : result),
  });
  q.add(['a', 'bad']);
  q.add(['c']);
  await q.idle();
  assert.equal(maxRunning, 1);
  assert.deepEqual(done, ['A', 'bad:broken photo', 'C']);
  assert.deepEqual(progress.slice(-1), ['3/3']);
  q.add(['d']);
  await q.idle();
  assert.deepEqual(done.at(-1), 'D');
  assert.deepEqual(progress.slice(-1), ['4/4']);
});

test('a paused queue waits until resumed', async () => {
  const seen = [];
  const q = createQueue(async (j) => seen.push(j));
  q.pause();
  q.add([1, 2]);
  await tick(20);
  assert.deepEqual(seen, []);
  q.resume();
  await q.idle();
  assert.deepEqual(seen, [1, 2]);
});

test('progress is reported after the result handler has finished', async () => {
  const log = [];
  const q = createQueue(async (j) => j, {
    onResult: async (j) => {
      await tick(5);
      log.push(`result ${j}`);
    },
    onProgress: ({ done, total }) => log.push(`progress ${done}/${total}`),
  });
  q.add(['a']);
  await q.idle();
  assert.deepEqual(log, ['progress 0/1', 'result a', 'progress 1/1']);
});

test('reset starts the count again once the queue is idle', async () => {
  const progress = [];
  const q = createQueue(async (j) => j, { onProgress: ({ done, total }) => progress.push(`${done}/${total}`) });
  q.add(['a', 'b']);
  await q.idle();
  q.reset();
  q.add(['c']);
  await q.idle();
  assert.equal(progress.at(-1), '1/1');
});

test('retried jobs run again without growing the total', async () => {
  const progress = [];
  let fails = 1;
  const failed = [];
  const q = createQueue(
    async (j) => {
      if (j === 'b' && fails-- > 0) throw new Error('expired');
      return j;
    },
    { onResult: (j, r, e) => e && failed.push(j), onProgress: ({ done, total }) => progress.push(`${done}/${total}`) },
  );
  q.add(['a', 'b', 'c']);
  await q.idle();
  q.retry(failed.splice(0));
  await q.idle();
  assert.equal(progress.at(-1), '3/3');
});

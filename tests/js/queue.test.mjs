import test from 'node:test';
import assert from 'node:assert/strict';
import { createQueue } from '../../app/queue.js';

const tick = (ms) => new Promise((r) => setTimeout(r, ms));

test('adding jobs does not start them; start runs them one at a time in order', async () => {
  let running = 0;
  let maxRunning = 0;
  const ran = [];
  const done = [];
  const progress = [];
  const q = createQueue(async (job) => {
    running++;
    maxRunning = Math.max(maxRunning, running);
    ran.push(job);
    await tick(5);
    running--;
    if (job === 'bad') throw new Error('broken photo');
    return job.toUpperCase();
  }, {
    onProgress: (p) => progress.push(`${p.state} ${p.done}/${p.total} ready ${p.ready}`),
    onResult: (job, result, error) => done.push(error ? `${job}:${error.message}` : result),
  });
  assert.equal(q.state, 'empty');
  q.add(['a', 'bad']);
  q.add(['c']);
  await tick(20);
  assert.deepEqual(ran, []);
  assert.equal(q.state, 'ready');
  assert.equal(q.ready, 3);
  assert.equal(q.start(), true);
  assert.equal(q.state, 'running');
  assert.equal(q.start(), false, 'start is ignored while running');
  await q.idle();
  assert.equal(maxRunning, 1);
  assert.deepEqual(done, ['A', 'bad:broken photo', 'C']);
  assert.equal(q.state, 'done');
  assert.deepEqual(progress, [
    'ready 0/0 ready 2',
    'ready 0/0 ready 3',
    'running 0/3 ready 0',
    'running 1/3 ready 0',
    'running 2/3 ready 0',
    'done 3/3 ready 0',
  ]);
});

test('start with nothing ready does nothing', async () => {
  const q = createQueue(async () => assert.fail('should not run'));
  assert.equal(q.start(), false);
  assert.equal(q.state, 'empty');
  q.add([]);
  assert.equal(q.start(), false);
});

test('photos added during a run wait for the next start and are counted in their own run', async () => {
  const ran = [];
  const progress = [];
  const q = createQueue(async (j) => {
    ran.push(j);
    await tick(5);
  }, { onProgress: (p) => progress.push(`${p.state} ${p.done}/${p.total}`) });
  q.add(['a', 'b']);
  q.start();
  q.add(['c']);
  await q.idle();
  assert.deepEqual(ran, ['a', 'b']);
  assert.equal(q.state, 'ready');
  assert.equal(progress.at(-1), 'ready 2/2');
  q.start();
  await q.idle();
  assert.deepEqual(ran, ['a', 'b', 'c']);
  assert.equal(progress.at(-1), 'done 1/1');
});

test('a ready job can be removed', async () => {
  const ran = [];
  const q = createQueue(async (j) => ran.push(j));
  q.add(['a', 'b', 'c']);
  assert.equal(q.remove('b'), true);
  assert.equal(q.remove('x'), false);
  assert.equal(q.ready, 2);
  q.start();
  await q.idle();
  assert.deepEqual(ran, ['a', 'c']);
  q.add(['d']);
  assert.equal(q.remove('d'), true);
  assert.equal(q.state, 'done', 'removing the last ready photo goes back to the finished run');
});

test('a paused queue waits until resumed', async () => {
  const seen = [];
  const q = createQueue(async (j) => seen.push(j));
  q.add([1, 2]);
  q.start();
  q.pause();
  await tick(20);
  assert.deepEqual(seen, [1]);
  assert.equal(q.state, 'paused');
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
  q.start();
  await q.idle();
  assert.deepEqual(log, ['progress 0/0', 'progress 0/1', 'result a', 'progress 1/1']);
});

test('reset clears everything once the queue is idle', async () => {
  const progress = [];
  const q = createQueue(async (j) => j, { onProgress: ({ done, total, state }) => progress.push(`${state} ${done}/${total}`) });
  q.add(['a', 'b']);
  q.start();
  await q.idle();
  q.add(['x']);
  q.reset();
  assert.equal(q.state, 'empty');
  assert.equal(q.ready, 0);
  q.add(['c']);
  q.start();
  await q.idle();
  assert.equal(progress.at(-1), 'done 1/1');
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
  q.start();
  await q.idle();
  q.retry(failed.splice(0));
  await q.idle();
  assert.equal(progress.at(-1), '3/3');
  assert.equal(q.state, 'done');
});

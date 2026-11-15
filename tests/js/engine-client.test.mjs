import test from 'node:test';
import assert from 'node:assert/strict';
import { createEngineClient } from '../../app/engine-client.js';

// A stand-in worker: answers each message with what `reply` returns, or crashes when told to.
function fakeWorkers(reply) {
  const made = [];
  const makeWorker = () => {
    const w = {
      terminated: false,
      terminate() {
        this.terminated = true;
      },
      postMessage(msg) {
        setTimeout(() => (msg.kind === 'crash' ? this.onerror({}) : this.onmessage({ data: { id: msg.id, ...reply(msg) } })));
      },
    };
    made.push(w);
    return w;
  };
  return { made, makeWorker };
}

test('results come back for the right photo', async () => {
  const { makeWorker } = fakeWorkers((m) => ({ ok: true, w: 10, h: 10, echo: m.blob }));
  const c = createEngineClient({ makeWorker });
  const [a, b] = await Promise.all([c.correct('a', 'ok'), c.correct('b', 'ok')]);
  assert.equal(a.echo, 'a');
  assert.equal(b.echo, 'b');
});

test('the worker is replaced after a big photo and after a crash', async () => {
  const { made, makeWorker } = fakeWorkers(() => ({ ok: true, w: 5000, h: 4000 }));
  const c = createEngineClient({ makeWorker });
  await c.correct('big', 'ok');
  assert.equal(made.length, 2);
  assert.equal(made[0].terminated, true);
  const r = await c.correct('x', 'crash');
  assert.deepEqual([r.ok, made.length], [false, 3]);
  assert.match(r.message, /engine stopped/);
});

test('stage messages reach onStage and do not end the call', async () => {
  const made = [];
  const makeWorker = () => {
    const w = {
      terminate() {},
      postMessage(msg) {
        setTimeout(() => {
          for (const stage of ['reading', 'correcting', 'saving']) this.onmessage({ data: { id: msg.id, stage } });
          this.onmessage({ data: { id: msg.id, ok: true, w: 1, h: 1 } });
        });
      },
    };
    made.push(w);
    return w;
  };
  const stages = [];
  const r = await createEngineClient({ makeWorker }).correct('a', 'ok', (s) => stages.push(s));
  assert.deepEqual(stages, ['reading', 'correcting', 'saving']);
  assert.equal(r.ok, true);
});

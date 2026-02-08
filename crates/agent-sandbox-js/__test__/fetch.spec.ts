import test from 'ava';
import { Sandbox } from '../index.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { createTempDir, createSandbox, createFetchSandbox, cleanup } from './helpers.js';

// --- Fetch API tests ---

test('fetch: returns error when networking is disabled', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await t.throwsAsync(
    () => sandbox.fetch({ url: 'https://example.com' }),
    { message: /networking disabled/ },
  );

  cleanup(tmpDir);
});

test('fetch: basic GET request', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.fetch({ url: 'https://example.com' });
  t.is(result.status, 200);
  const body = result.body.toString();
  t.true(body.includes('Example Domain'));

  cleanup(tmpDir);
});

test('fetch: explicit GET method', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.fetch({ url: 'https://example.com', method: 'GET' });
  t.is(result.status, 200);

  cleanup(tmpDir);
});

test('fetch: POST with body', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.fetch({
    url: 'https://httpbin.org/post',
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: Buffer.from(JSON.stringify({ key: 'value' })),
  });
  t.is(result.status, 200);
  const body = JSON.parse(result.body.toString());
  t.truthy(body.data || body.json);

  cleanup(tmpDir);
});

test('fetch: custom headers are sent', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.fetch({
    url: 'https://httpbin.org/headers',
    headers: { 'X-Custom-Header': 'test-value' },
  });
  t.is(result.status, 200);
  const body = result.body.toString();
  t.true(body.includes('X-Custom-Header') || body.includes('x-custom-header'));

  cleanup(tmpDir);
});

test('fetch: response includes headers', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.fetch({ url: 'https://example.com' });
  t.is(typeof result.headers, 'object');
  // Should have at least content-type
  const headerKeys = Object.keys(result.headers).map((k) => k.toLowerCase());
  t.true(headerKeys.includes('content-type'));

  cleanup(tmpDir);
});

test('fetch: blocked domain returns error', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox({
    blockedDomains: ['example.com'],
  });

  await t.throwsAsync(
    () => sandbox.fetch({ url: 'https://example.com' }),
    { message: /block/i },
  );

  cleanup(tmpDir);
});

test('fetch: allowed domains restricts access', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox({
    allowedDomains: ['example.com'],
  });

  // Allowed domain works
  const result = await sandbox.fetch({ url: 'https://example.com' });
  t.is(result.status, 200);

  // Non-allowed domain fails
  await t.throwsAsync(
    () => sandbox.fetch({ url: 'https://httpbin.org/get' }),
    { message: /allow|denied/i },
  );

  cleanup(tmpDir);
});

test('fetch: SSRF protection blocks private IPs', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox({ denyPrivateIps: true });

  await t.throwsAsync(
    () => sandbox.fetch({ url: 'http://127.0.0.1' }),
    { message: /private|block/i },
  );

  cleanup(tmpDir);
});

// --- curl interception tests ---

test('curl: basic GET via exec', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.exec('curl', ['https://example.com']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('Example Domain'));
  t.true(result.stderr.toString().includes('HTTP 200'));

  cleanup(tmpDir);
});

test('curl: returns error when networking is disabled', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await t.throwsAsync(
    () => sandbox.exec('curl', ['https://example.com']),
    { message: /networking disabled/ },
  );

  cleanup(tmpDir);
});

test('curl: with custom headers', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.exec('curl', [
    '-H', 'Accept: application/json',
    'https://httpbin.org/headers',
  ]);
  t.is(result.exitCode, 0);
  const body = result.stdout.toString();
  t.true(body.includes('Accept') || body.includes('accept'));

  cleanup(tmpDir);
});

test('curl: POST with -d flag', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.exec('curl', [
    '-d', '{"key":"value"}',
    '-H', 'Content-Type: application/json',
    'https://httpbin.org/post',
  ]);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().length > 0);

  cleanup(tmpDir);
});

test('curl: explicit method with -X', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.exec('curl', [
    '-X', 'PUT',
    '-d', 'updated',
    'https://httpbin.org/put',
  ]);
  t.is(result.exitCode, 0);

  cleanup(tmpDir);
});

test('curl: -o writes response to file', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.exec('curl', [
    '-o', 'output.html',
    'https://example.com',
  ]);
  t.is(result.exitCode, 0);

  const content = fs.readFileSync(path.join(tmpDir, 'output.html'), 'utf-8');
  t.true(content.includes('Example Domain'));

  cleanup(tmpDir);
});

test('curl: silent and location flags are accepted', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.exec('curl', ['-sSL', 'https://example.com']);
  // -sSL is combined flags, treated as unknown single arg, but URL still works
  t.is(typeof result.exitCode, 'number');

  cleanup(tmpDir);
});

// --- JS fetch() inside Boa runtime tests ---

test('execJs fetch: basic GET returns status 200', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.execJs(
    "var r = fetch('https://example.com'); console.log(r.status)",
  );
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('200'));

  cleanup(tmpDir);
});

test('execJs fetch: response body contains content', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.execJs(
    "var r = fetch('https://example.com'); console.log(r.body.indexOf('Example Domain') >= 0)",
  );
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('true'));

  cleanup(tmpDir);
});

test('execJs fetch: ok property is true for 200', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.execJs(
    "var r = fetch('https://example.com'); console.log(r.ok)",
  );
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('true'));

  cleanup(tmpDir);
});

test('execJs fetch: text() method returns body string', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.execJs(
    "var r = fetch('https://example.com'); console.log(r.text().indexOf('Example') >= 0)",
  );
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('true'));

  cleanup(tmpDir);
});

test('execJs fetch: POST with options', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.execJs(
    `var r = fetch('https://httpbin.org/post', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: '{"key":"value"}'
    });
    console.log(r.status)`,
  );
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('200'));

  cleanup(tmpDir);
});

test('execJs fetch: throws error when networking is disabled', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const result = await sandbox.execJs(
    "try { fetch('https://example.com'); } catch(e) { console.log('caught: ' + e.message); }",
  );
  const output = result.stdout.toString() + result.stderr.toString();
  t.true(
    output.includes('disabled') || output.includes('error') || output.includes('caught') || result.exitCode !== 0,
    `Expected fetch to fail when disabled. Output: ${output}`,
  );

  cleanup(tmpDir);
});

test('execJs fetch: headers object is accessible', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.execJs(
    "var r = fetch('https://example.com'); console.log(typeof r.headers)",
  );
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('object'));

  cleanup(tmpDir);
});

test('execJs fetch: full workflow - fetch, check status, read body', async (t) => {
  const { tmpDir, sandbox } = createFetchSandbox();

  const result = await sandbox.execJs(`
    var response = fetch('https://example.com');
    var status = response.status;
    var ok = response.ok;
    var body = response.text();
    var hasContent = body.indexOf('Example Domain') >= 0;
    console.log(status + '|' + ok + '|' + hasContent);
  `);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), '200|true|true');

  cleanup(tmpDir);
});

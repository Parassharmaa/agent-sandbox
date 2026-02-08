import test from 'ava';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { createTempDir, createSandbox, cleanup } from './helpers.js';

test('node --version returns version string', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['--version']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('node v0.1.0'));
  cleanup(tmpDir);
});

test('node -e evaluates inline JavaScript', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['-e', "console.log('hello from js')"]);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('hello from js'));
  cleanup(tmpDir);
});

test('node -p evaluates and prints result', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['-p', '2 + 3 * 4']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), '14');
  cleanup(tmpDir);
});

test('node runs script from file', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'script.js'), 'var x = 10; var y = 20; console.log(x + y);');
  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('node', ['/work/script.js']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('30'));
  cleanup(tmpDir);
});

test('node handles syntax errors', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['-e', 'function {']);
  t.not(result.exitCode, 0);
  t.true(result.stderr.toString().length > 0);
  cleanup(tmpDir);
});

test('node handles runtime errors', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['-e', "throw new Error('oops')"]);
  t.not(result.exitCode, 0);
  t.true(result.stderr.toString().includes('oops'));
  cleanup(tmpDir);
});

test('node JSON operations', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['-p', 'JSON.stringify({a: 1, b: [2,3]})']);
  t.is(result.exitCode, 0);
  const parsed = JSON.parse(result.stdout.toString().trim());
  t.deepEqual(parsed, { a: 1, b: [2, 3] });
  cleanup(tmpDir);
});

test('node array methods', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['-p', "[5,3,1,4,2].sort().join(',')"])
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), '1,2,3,4,5');
  cleanup(tmpDir);
});

test('node template literals', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['-e', "const name = 'World'; console.log(`Hello ${name}!`)"]);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('Hello World!'));
  cleanup(tmpDir);
});

test('node fibonacci script', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(
    path.join(tmpDir, 'fib.js'),
    `function fib(n) {
      if (n <= 1) return n;
      return fib(n - 1) + fib(n - 2);
    }
    console.log(fib(10));`
  );
  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('node', ['/work/fib.js']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('55'));
  cleanup(tmpDir);
});

test('execJs convenience method works', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.execJs("console.log('execJs works')");
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('execJs works'));
  cleanup(tmpDir);
});

test('execJs evaluates complex expressions', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.execJs(`
    const data = [1, 2, 3, 4, 5];
    const sum = data.reduce((a, b) => a + b, 0);
    const avg = sum / data.length;
    console.log('sum=' + sum + ' avg=' + avg);
  `);
  t.is(result.exitCode, 0);
  const output = result.stdout.toString();
  t.true(output.includes('sum=15'));
  t.true(output.includes('avg=3'));
  cleanup(tmpDir);
});

test('execJs returns error for invalid code', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.execJs("throw new Error('test error')");
  t.not(result.exitCode, 0);
  t.true(result.stderr.toString().includes('test error'));
  cleanup(tmpDir);
});

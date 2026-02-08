import test from 'ava';
import { Sandbox } from '../index.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { createTempDir, createSandbox, cleanup } from './helpers.js';

test('security: path traversal variants are blocked', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const traversals = [
    '../../../etc/passwd',
    '../../etc/shadow',
    'foo/../../..',
    './../../etc/hosts',
    'foo/../../../etc/passwd',
  ];

  for (const p of traversals) {
    await t.throwsAsync(() => sandbox.readFile(p), { message: /traversal/ });
  }

  cleanup(tmpDir);
});

test('security: writeFile traversal is blocked', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await t.throwsAsync(
    () => sandbox.writeFile('../../../tmp/escape.txt', Buffer.from('pwned')),
    { message: /traversal/ },
  );

  cleanup(tmpDir);
});

test('security: listDir traversal is blocked', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await t.throwsAsync(() => sandbox.listDir('../../../etc'), {
    message: /traversal/,
  });

  cleanup(tmpDir);
});

test('security: symlink escape is blocked', async (t) => {
  const tmpDir = createTempDir();

  // Create sandbox first, then add the symlink after snapshotting
  const sandbox = new Sandbox({ workDir: tmpDir });

  // Now create a symlink inside work dir pointing outside
  fs.symlinkSync('/etc', path.join(tmpDir, 'escape_link'));

  // Reading via the symlink should fail — resolved path is outside the sandbox
  try {
    await sandbox.readFile('escape_link/passwd');
    t.fail('Should have thrown an error for symlink escape');
  } catch (err: any) {
    t.true(
      err.message.includes('traversal') || err.message.includes('Permission denied'),
      `Expected traversal or permission error, got: ${err.message}`,
    );
  }

  cleanup(tmpDir);
});

test('security: cat cannot read host files', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const result = await sandbox.exec('cat', ['/etc/passwd']);
  t.not(result.exitCode, 0);
  t.is(result.stdout.toString(), '');

  cleanup(tmpDir);
});

test('security: find confined to sandbox', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const result = await sandbox.exec('find', ['/', '-name', 'passwd']);
  const output = result.stdout.toString();
  t.false(output.includes('/etc/passwd'), 'find should not see /etc/passwd');

  cleanup(tmpDir);
});

test('security: cp cannot write outside sandbox', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'secret.txt'), 'data');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('cp', ['/work/secret.txt', '/tmp/escape.txt']);

  t.not(result.exitCode, 0);
  t.false(fs.existsSync('/tmp/escape.txt'));

  cleanup(tmpDir);
});

test('security: grep cannot read host files', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const result = await sandbox.exec('grep', ['root', '/etc/passwd']);
  t.not(result.exitCode, 0);

  cleanup(tmpDir);
});

test('security: rm cannot delete outside sandbox', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const result = await sandbox.exec('rm', ['/etc/hostname']);
  t.not(result.exitCode, 0);

  cleanup(tmpDir);
});

test('security: env vars are isolated from host', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({
    workDir: tmpDir,
    envVars: { SECRET_KEY: 's3cret' },
  });

  const result = await sandbox.exec('env', []);
  const output = result.stdout.toString();

  t.true(output.includes('SECRET_KEY=s3cret'));
  t.false(output.includes('HOME='), 'Host HOME should not leak');
  t.false(output.includes('USER='), 'Host USER should not leak');

  cleanup(tmpDir);
});

test('security: destroyed sandbox blocks all operations', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await sandbox.destroy();

  await t.throwsAsync(() => sandbox.readFile('any.txt'), { message: /destroyed/ });
  await t.throwsAsync(() => sandbox.writeFile('any.txt', Buffer.from('data')), {
    message: /destroyed/,
  });
  await t.throwsAsync(() => sandbox.listDir('.'), { message: /destroyed/ });
  await t.throwsAsync(() => sandbox.exec('echo', ['hi']), { message: /destroyed/ });
  await t.throwsAsync(() => sandbox.diff(), { message: /destroyed/ });

  cleanup(tmpDir);
});

test('security: multiple sandboxes are isolated', async (t) => {
  const tmpDir1 = createTempDir();
  const tmpDir2 = createTempDir();

  const sandbox1 = new Sandbox({ workDir: tmpDir1 });
  const sandbox2 = new Sandbox({ workDir: tmpDir2 });

  fs.writeFileSync(path.join(tmpDir1, 'secret.txt'), 'sandbox1 secret');

  // Sandbox2 should not see sandbox1's files
  const r2 = await sandbox2.exec('cat', ['/work/secret.txt']);
  t.not(r2.exitCode, 0, 'Sandbox2 should not see sandbox1 files');

  // Sandbox1 should see its own file
  const r1 = await sandbox1.exec('cat', ['/work/secret.txt']);
  t.is(r1.exitCode, 0);
  t.true(r1.stdout.toString().includes('sandbox1 secret'));

  cleanup(tmpDir1, tmpDir2);
});

test('security: node cannot access host filesystem', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('node', ['/etc/passwd']);
  t.not(result.exitCode, 0);
  cleanup(tmpDir);
});

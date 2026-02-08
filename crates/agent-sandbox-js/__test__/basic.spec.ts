import test from 'ava';
import { Sandbox } from '../index.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { createTempDir, createSandbox, cleanup } from './helpers.js';

test('constructor creates sandbox with work directory', (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });
  t.truthy(sandbox);
  cleanup(tmpDir);
});

test('read_file returns file contents', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'test.txt'), 'hello world');

  const { sandbox } = createSandbox(tmpDir);
  const content = await sandbox.readFile('test.txt');
  t.is(content.toString(), 'hello world');

  cleanup(tmpDir);
});

test('write_file creates a file', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await sandbox.writeFile('output.txt', Buffer.from('test content'));
  const content = fs.readFileSync(path.join(tmpDir, 'output.txt'), 'utf-8');
  t.is(content, 'test content');

  cleanup(tmpDir);
});

test('list_dir returns directory entries', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'a.txt'), 'a');
  fs.writeFileSync(path.join(tmpDir, 'b.txt'), 'b');
  fs.mkdirSync(path.join(tmpDir, 'subdir'));

  const { sandbox } = createSandbox(tmpDir);
  const entries = await sandbox.listDir('.');
  t.is(entries.length, 3);
  t.truthy(entries.find((e) => e.name === 'a.txt' && e.isFile));
  t.truthy(entries.find((e) => e.name === 'subdir' && e.isDir));

  cleanup(tmpDir);
});

test('diff detects file changes', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'existing.txt'), 'original');

  const { sandbox } = createSandbox(tmpDir);

  // Create a new file
  fs.writeFileSync(path.join(tmpDir, 'new.txt'), 'new content');

  // Modify existing file
  fs.writeFileSync(path.join(tmpDir, 'existing.txt'), 'modified');

  const changes = await sandbox.diff();
  t.truthy(changes.find((c) => c.path === 'new.txt' && c.kind === 'created'));
  t.truthy(changes.find((c) => c.path === 'existing.txt' && c.kind === 'modified'));

  cleanup(tmpDir);
});

test('destroy prevents further operations', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await sandbox.destroy();

  await t.throwsAsync(() => sandbox.readFile('test.txt'), {
    message: /destroyed/,
  });

  cleanup(tmpDir);
});

test('path traversal is blocked', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await t.throwsAsync(() => sandbox.readFile('../../../etc/passwd'), {
    message: /traversal/,
  });

  cleanup(tmpDir);
});

test('exec command not found', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  await t.throwsAsync(() => sandbox.exec('nonexistent_cmd', []), {
    message: /not found/,
  });

  cleanup(tmpDir);
});

test('curl is listed in available tools', (t) => {
  const tools = Sandbox.availableTools();
  t.true(tools.includes('curl'));
});

test('node is listed in available tools', (t) => {
  const tools = Sandbox.availableTools();
  t.true(tools.includes('node'));
});

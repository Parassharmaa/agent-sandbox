import test from 'ava';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { createTempDir, createSandbox, cleanup } from './helpers.js';

test('exec runs cat command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'hello.txt'), 'hello sandbox');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('cat', ['/work/hello.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('hello sandbox'));

  cleanup(tmpDir);
});

test('exec runs ls command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'file1.txt'), '');
  fs.writeFileSync(path.join(tmpDir, 'file2.txt'), '');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('ls', ['/work']);
  t.is(result.exitCode, 0);
  const output = result.stdout.toString();
  t.true(output.includes('file1.txt'));
  t.true(output.includes('file2.txt'));

  cleanup(tmpDir);
});

test('exec runs echo command', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('echo', ['hello', 'world']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'hello world');

  cleanup(tmpDir);
});

test('exec runs grep command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'code.rs'), 'fn main() {\n    println!("hello");\n}\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('grep', ['main', '/work/code.rs']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('fn main()'));

  cleanup(tmpDir);
});

test('exec runs find command', async (t) => {
  const tmpDir = createTempDir();
  fs.mkdirSync(path.join(tmpDir, 'a', 'b'), { recursive: true });
  fs.writeFileSync(path.join(tmpDir, 'a', 'b', 'deep.txt'), 'deep');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('find', ['/work', '-name', '*.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('deep.txt'));

  cleanup(tmpDir);
});

test('exec runs wc command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'lines.txt'), 'one\ntwo\nthree\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('wc', ['-l', '/work/lines.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('3'));

  cleanup(tmpDir);
});

test('exec runs sed command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'input.txt'), 'hello world\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('sed', ['s/world/rust/g', '/work/input.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('hello rust'));

  cleanup(tmpDir);
});

test('exec runs head command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'data.txt'), 'line1\nline2\nline3\nline4\nline5\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('head', ['-n', '2', '/work/data.txt']);
  t.is(result.exitCode, 0);
  const output = result.stdout.toString();
  t.true(output.includes('line1'));
  t.true(output.includes('line2'));
  t.false(output.includes('line3'));

  cleanup(tmpDir);
});

test('exec runs tail command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'data.txt'), 'line1\nline2\nline3\nline4\nline5\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('tail', ['-n', '2', '/work/data.txt']);
  t.is(result.exitCode, 0);
  const output = result.stdout.toString();
  t.false(output.includes('line3'));
  t.true(output.includes('line4'));
  t.true(output.includes('line5'));

  cleanup(tmpDir);
});

test('exec runs sort command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'unsorted.txt'), 'banana\napple\ncherry\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('sort', ['/work/unsorted.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'apple\nbanana\ncherry');

  cleanup(tmpDir);
});

test('exec runs uniq command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'dups.txt'), 'a\na\nb\nb\nb\nc\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('uniq', ['/work/dups.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'a\nb\nc');

  cleanup(tmpDir);
});

test('exec runs basename and dirname commands', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const bn = await sandbox.exec('basename', ['/work/path/to/file.txt']);
  t.is(bn.exitCode, 0);
  t.is(bn.stdout.toString().trim(), 'file.txt');

  const dn = await sandbox.exec('dirname', ['/work/path/to/file.txt']);
  t.is(dn.exitCode, 0);
  t.is(dn.stdout.toString().trim(), '/work/path/to');

  cleanup(tmpDir);
});

test('exec runs mkdir and touch commands', async (t) => {
  const { tmpDir, sandbox } = createSandbox();

  const mkResult = await sandbox.exec('mkdir', ['-p', '/work/newdir/sub']);
  t.is(mkResult.exitCode, 0);

  const touchResult = await sandbox.exec('touch', ['/work/newdir/sub/file.txt']);
  t.is(touchResult.exitCode, 0);

  const lsResult = await sandbox.exec('ls', ['/work/newdir/sub']);
  t.is(lsResult.exitCode, 0);
  t.true(lsResult.stdout.toString().includes('file.txt'));

  cleanup(tmpDir);
});

test('exec runs cp command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'src.txt'), 'copy me');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('cp', ['/work/src.txt', '/work/dst.txt']);
  t.is(result.exitCode, 0);

  const content = fs.readFileSync(path.join(tmpDir, 'dst.txt'), 'utf-8');
  t.is(content, 'copy me');

  cleanup(tmpDir);
});

test('exec runs mv command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'old.txt'), 'move me');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('mv', ['/work/old.txt', '/work/new.txt']);
  t.is(result.exitCode, 0);

  t.false(fs.existsSync(path.join(tmpDir, 'old.txt')));
  t.is(fs.readFileSync(path.join(tmpDir, 'new.txt'), 'utf-8'), 'move me');

  cleanup(tmpDir);
});

test('exec runs rm command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'delete.txt'), 'bye');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('rm', ['/work/delete.txt']);
  t.is(result.exitCode, 0);
  t.false(fs.existsSync(path.join(tmpDir, 'delete.txt')));

  cleanup(tmpDir);
});

test('exec runs base64 encode', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'plain.txt'), 'hello');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('base64', ['/work/plain.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'aGVsbG8=');

  cleanup(tmpDir);
});

test('exec runs sha256sum command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'hash.txt'), 'hello');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('sha256sum', ['/work/hash.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824'));

  cleanup(tmpDir);
});

test('exec runs diff command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'a.txt'), 'line1\nline2\nline3\n');
  fs.writeFileSync(path.join(tmpDir, 'b.txt'), 'line1\nmodified\nline3\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('diff', ['/work/a.txt', '/work/b.txt']);
  // diff returns exit code 1 when files differ
  t.true(result.exitCode === 1);
  const output = result.stdout.toString();
  t.true(output.includes('line2') || output.includes('modified'));

  cleanup(tmpDir);
});

test('exec runs cut command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'csv.txt'), 'a,b,c\n1,2,3\n');

  const { sandbox } = createSandbox(tmpDir);
  const result = await sandbox.exec('cut', ['-d', ',', '-f', '2', '/work/csv.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'b\n2');

  cleanup(tmpDir);
});

test('exec runs env command', async (t) => {
  const { tmpDir, sandbox } = createSandbox();
  const result = await sandbox.exec('env', []);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('TOOLBOX_CMD=env'));

  cleanup(tmpDir);
});

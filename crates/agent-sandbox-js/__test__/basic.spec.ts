import test from 'ava';
import { Sandbox } from '../index.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';

function createTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'agent-sandbox-test-'));
}

test('constructor creates sandbox with work directory', (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });
  t.truthy(sandbox);
  fs.rmSync(tmpDir, { recursive: true });
});

test('read_file returns file contents', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'test.txt'), 'hello world');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const content = await sandbox.readFile('test.txt');
  t.is(content.toString(), 'hello world');

  fs.rmSync(tmpDir, { recursive: true });
});

test('write_file creates a file', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  await sandbox.writeFile('output.txt', Buffer.from('test content'));
  const content = fs.readFileSync(path.join(tmpDir, 'output.txt'), 'utf-8');
  t.is(content, 'test content');

  fs.rmSync(tmpDir, { recursive: true });
});

test('list_dir returns directory entries', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'a.txt'), 'a');
  fs.writeFileSync(path.join(tmpDir, 'b.txt'), 'b');
  fs.mkdirSync(path.join(tmpDir, 'subdir'));

  const sandbox = new Sandbox({ workDir: tmpDir });
  const entries = await sandbox.listDir('.');
  t.is(entries.length, 3);
  t.truthy(entries.find((e) => e.name === 'a.txt' && e.isFile));
  t.truthy(entries.find((e) => e.name === 'subdir' && e.isDir));

  fs.rmSync(tmpDir, { recursive: true });
});

test('diff detects file changes', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'existing.txt'), 'original');

  const sandbox = new Sandbox({ workDir: tmpDir });

  // Create a new file
  fs.writeFileSync(path.join(tmpDir, 'new.txt'), 'new content');

  // Modify existing file
  fs.writeFileSync(path.join(tmpDir, 'existing.txt'), 'modified');

  const changes = await sandbox.diff();
  t.truthy(changes.find((c) => c.path === 'new.txt' && c.kind === 'created'));
  t.truthy(changes.find((c) => c.path === 'existing.txt' && c.kind === 'modified'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs cat command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'hello.txt'), 'hello sandbox');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('cat', ['/work/hello.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('hello sandbox'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs ls command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'file1.txt'), '');
  fs.writeFileSync(path.join(tmpDir, 'file2.txt'), '');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('ls', ['/work']);
  t.is(result.exitCode, 0);
  const output = result.stdout.toString();
  t.true(output.includes('file1.txt'));
  t.true(output.includes('file2.txt'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs echo command', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('echo', ['hello', 'world']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'hello world');

  fs.rmSync(tmpDir, { recursive: true });
});

test('destroy prevents further operations', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  await sandbox.destroy();

  await t.throwsAsync(() => sandbox.readFile('test.txt'), {
    message: /destroyed/,
  });

  fs.rmSync(tmpDir, { recursive: true });
});

test('path traversal is blocked', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  await t.throwsAsync(() => sandbox.readFile('../../../etc/passwd'), {
    message: /traversal/,
  });

  fs.rmSync(tmpDir, { recursive: true });
});

// --- exec tests for additional tools ---

test('exec runs grep command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'code.rs'), 'fn main() {\n    println!("hello");\n}\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('grep', ['main', '/work/code.rs']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('fn main()'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs find command', async (t) => {
  const tmpDir = createTempDir();
  fs.mkdirSync(path.join(tmpDir, 'a', 'b'), { recursive: true });
  fs.writeFileSync(path.join(tmpDir, 'a', 'b', 'deep.txt'), 'deep');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('find', ['/work', '-name', '*.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('deep.txt'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs wc command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'lines.txt'), 'one\ntwo\nthree\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('wc', ['-l', '/work/lines.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('3'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs sed command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'input.txt'), 'hello world\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('sed', ['s/world/rust/g', '/work/input.txt']);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('hello rust'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs head command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'data.txt'), 'line1\nline2\nline3\nline4\nline5\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('head', ['-n', '2', '/work/data.txt']);
  t.is(result.exitCode, 0);
  const output = result.stdout.toString();
  t.true(output.includes('line1'));
  t.true(output.includes('line2'));
  t.false(output.includes('line3'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs tail command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'data.txt'), 'line1\nline2\nline3\nline4\nline5\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('tail', ['-n', '2', '/work/data.txt']);
  t.is(result.exitCode, 0);
  const output = result.stdout.toString();
  t.false(output.includes('line3'));
  t.true(output.includes('line4'));
  t.true(output.includes('line5'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs sort command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'unsorted.txt'), 'banana\napple\ncherry\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('sort', ['/work/unsorted.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'apple\nbanana\ncherry');

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs uniq command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'dups.txt'), 'a\na\nb\nb\nb\nc\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('uniq', ['/work/dups.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'a\nb\nc');

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs basename and dirname commands', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  const bn = await sandbox.exec('basename', ['/work/path/to/file.txt']);
  t.is(bn.exitCode, 0);
  t.is(bn.stdout.toString().trim(), 'file.txt');

  const dn = await sandbox.exec('dirname', ['/work/path/to/file.txt']);
  t.is(dn.exitCode, 0);
  t.is(dn.stdout.toString().trim(), '/work/path/to');

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs mkdir and touch commands', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  const mkResult = await sandbox.exec('mkdir', ['-p', '/work/newdir/sub']);
  t.is(mkResult.exitCode, 0);

  const touchResult = await sandbox.exec('touch', ['/work/newdir/sub/file.txt']);
  t.is(touchResult.exitCode, 0);

  const lsResult = await sandbox.exec('ls', ['/work/newdir/sub']);
  t.is(lsResult.exitCode, 0);
  t.true(lsResult.stdout.toString().includes('file.txt'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs cp command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'src.txt'), 'copy me');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('cp', ['/work/src.txt', '/work/dst.txt']);
  t.is(result.exitCode, 0);

  const content = fs.readFileSync(path.join(tmpDir, 'dst.txt'), 'utf-8');
  t.is(content, 'copy me');

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs mv command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'old.txt'), 'move me');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('mv', ['/work/old.txt', '/work/new.txt']);
  t.is(result.exitCode, 0);

  t.false(fs.existsSync(path.join(tmpDir, 'old.txt')));
  t.is(fs.readFileSync(path.join(tmpDir, 'new.txt'), 'utf-8'), 'move me');

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs rm command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'delete.txt'), 'bye');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('rm', ['/work/delete.txt']);
  t.is(result.exitCode, 0);
  t.false(fs.existsSync(path.join(tmpDir, 'delete.txt')));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs base64 encode', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'plain.txt'), 'hello');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('base64', ['/work/plain.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'aGVsbG8=');

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs sha256sum command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'hash.txt'), 'hello');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('sha256sum', ['/work/hash.txt']);
  t.is(result.exitCode, 0);
  // sha256 of "hello" is 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
  t.true(result.stdout.toString().includes('2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs diff command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'a.txt'), 'line1\nline2\nline3\n');
  fs.writeFileSync(path.join(tmpDir, 'b.txt'), 'line1\nmodified\nline3\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('diff', ['/work/a.txt', '/work/b.txt']);
  // diff returns exit code 1 when files differ
  t.true(result.exitCode === 1);
  const output = result.stdout.toString();
  t.true(output.includes('line2') || output.includes('modified'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs cut command', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'csv.txt'), 'a,b,c\n1,2,3\n');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('cut', ['-d', ',', '-f', '2', '/work/csv.txt']);
  t.is(result.exitCode, 0);
  t.is(result.stdout.toString().trim(), 'b\n2');

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec runs env command', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('env', []);
  t.is(result.exitCode, 0);
  t.true(result.stdout.toString().includes('TOOLBOX_CMD=env'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('exec command not found', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  await t.throwsAsync(() => sandbox.exec('nonexistent_cmd', []), {
    message: /not found/,
  });

  fs.rmSync(tmpDir, { recursive: true });
});

// --- Security tests ---

test('security: path traversal variants are blocked', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

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

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: writeFile traversal is blocked', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  await t.throwsAsync(
    () => sandbox.writeFile('../../../tmp/escape.txt', Buffer.from('pwned')),
    { message: /traversal/ },
  );

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: listDir traversal is blocked', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  await t.throwsAsync(() => sandbox.listDir('../../../etc'), {
    message: /traversal/,
  });

  fs.rmSync(tmpDir, { recursive: true });
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

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: cat cannot read host files', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  const result = await sandbox.exec('cat', ['/etc/passwd']);
  t.not(result.exitCode, 0);
  t.is(result.stdout.toString(), '');

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: find confined to sandbox', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  const result = await sandbox.exec('find', ['/', '-name', 'passwd']);
  const output = result.stdout.toString();
  t.false(output.includes('/etc/passwd'), 'find should not see /etc/passwd');

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: cp cannot write outside sandbox', async (t) => {
  const tmpDir = createTempDir();
  fs.writeFileSync(path.join(tmpDir, 'secret.txt'), 'data');

  const sandbox = new Sandbox({ workDir: tmpDir });
  const result = await sandbox.exec('cp', ['/work/secret.txt', '/tmp/escape.txt']);

  t.not(result.exitCode, 0);
  t.false(fs.existsSync('/tmp/escape.txt'));

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: grep cannot read host files', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  const result = await sandbox.exec('grep', ['root', '/etc/passwd']);
  t.not(result.exitCode, 0);

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: rm cannot delete outside sandbox', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  const result = await sandbox.exec('rm', ['/etc/hostname']);
  t.not(result.exitCode, 0);

  fs.rmSync(tmpDir, { recursive: true });
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

  fs.rmSync(tmpDir, { recursive: true });
});

test('security: destroyed sandbox blocks all operations', async (t) => {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({ workDir: tmpDir });

  await sandbox.destroy();

  await t.throwsAsync(() => sandbox.readFile('any.txt'), { message: /destroyed/ });
  await t.throwsAsync(() => sandbox.writeFile('any.txt', Buffer.from('data')), {
    message: /destroyed/,
  });
  await t.throwsAsync(() => sandbox.listDir('.'), { message: /destroyed/ });
  await t.throwsAsync(() => sandbox.exec('echo', ['hi']), { message: /destroyed/ });
  await t.throwsAsync(() => sandbox.diff(), { message: /destroyed/ });

  fs.rmSync(tmpDir, { recursive: true });
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

  fs.rmSync(tmpDir1, { recursive: true });
  fs.rmSync(tmpDir2, { recursive: true });
});

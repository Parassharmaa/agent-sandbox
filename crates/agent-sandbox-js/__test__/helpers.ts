import { Sandbox } from '../index.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';

export function createTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'agent-sandbox-test-'));
}

export function createSandbox(tmpDir?: string): { tmpDir: string; sandbox: InstanceType<typeof Sandbox> } {
  const dir = tmpDir ?? createTempDir();
  const sandbox = new Sandbox({ workDir: dir });
  return { tmpDir: dir, sandbox };
}

export function createFetchSandbox(opts?: {
  allowedDomains?: string[];
  blockedDomains?: string[];
  denyPrivateIps?: boolean;
}): { tmpDir: string; sandbox: InstanceType<typeof Sandbox> } {
  const tmpDir = createTempDir();
  const sandbox = new Sandbox({
    workDir: tmpDir,
    fetchPolicy: {
      allowedDomains: opts?.allowedDomains,
      blockedDomains: opts?.blockedDomains,
      denyPrivateIps: opts?.denyPrivateIps ?? true,
    },
  });
  return { tmpDir, sandbox };
}

export function cleanup(...dirs: string[]) {
  for (const dir of dirs) {
    fs.rmSync(dir, { recursive: true });
  }
}

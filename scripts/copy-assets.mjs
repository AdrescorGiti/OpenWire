import { cp, mkdir, rm } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const src = join(root, 'src');
const out = join(root, 'ui');

const jobs = [
  { from: join(src, 'js'), to: join(out, 'js') },
  { from: join(src, 'fonts'), to: join(out, 'fonts') },
  { from: join(root, 'app-icon.svg'), to: join(out, 'app-icon.svg') },
];

await mkdir(out, { recursive: true });

for (const { from, to } of jobs) {
  if (!existsSync(from)) continue;
  await rm(to, { recursive: true, force: true });
  await cp(from, to, { recursive: true });
  console.log('copied', from.replace(root + '/', ''), '->', to.replace(root + '/', ''));
}

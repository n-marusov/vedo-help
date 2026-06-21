import { mkdtempSync, rmdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test } from '@playwright/test';
import { fileInput, setActiveCollection, setupAuthAndCollection } from './helpers';

function createZipBuffer(files: { name: string; content: Buffer | string }[]): Buffer {
  const localHeaders: Buffer[] = [];
  const centralEntries: Buffer[] = [];
  const fileData: Buffer[] = [];
  let dataOffset = 0;

  for (const file of files) {
    const nameBuf = Buffer.from(file.name);
    const contentBuf = typeof file.content === 'string' ? Buffer.from(file.content) : file.content;
    const crc = crc32(contentBuf);

    const localHeader = Buffer.alloc(30);
    localHeader.writeUInt32LE(0x04034b50, 0);
    localHeader.writeUInt16LE(20, 4);
    localHeader.writeUInt16LE(0, 6);
    localHeader.writeUInt16LE(0, 8);
    localHeader.writeUInt16LE(0, 10);
    localHeader.writeUInt16LE(0, 12);
    localHeader.writeUInt32LE(crc, 14);
    localHeader.writeUInt32LE(contentBuf.length, 18);
    localHeader.writeUInt32LE(contentBuf.length, 22);
    localHeader.writeUInt16LE(nameBuf.length, 26);
    localHeader.writeUInt16LE(0, 28);
    localHeaders.push(Buffer.concat([localHeader, nameBuf]));
    fileData.push(contentBuf);

    const centralEntry = Buffer.alloc(46);
    centralEntry.writeUInt32LE(0x02014b50, 0);
    centralEntry.writeUInt16LE(20, 4);
    centralEntry.writeUInt16LE(20, 6);
    centralEntry.writeUInt16LE(0, 8);
    centralEntry.writeUInt16LE(0, 10);
    centralEntry.writeUInt16LE(0, 12);
    centralEntry.writeUInt16LE(0, 14);
    centralEntry.writeUInt32LE(crc, 16);
    centralEntry.writeUInt32LE(contentBuf.length, 20);
    centralEntry.writeUInt32LE(contentBuf.length, 24);
    centralEntry.writeUInt16LE(nameBuf.length, 28);
    centralEntry.writeUInt16LE(0, 30);
    centralEntry.writeUInt16LE(0, 32);
    centralEntry.writeUInt16LE(0, 34);
    centralEntry.writeUInt16LE(0, 36);
    centralEntry.writeUInt32LE(0, 38);
    centralEntry.writeUInt32LE(dataOffset, 42);
    centralEntries.push(Buffer.concat([centralEntry, nameBuf]));
    dataOffset += 30 + nameBuf.length + contentBuf.length;
  }

  const centralStart =
    localHeaders.reduce((sum, b) => sum + b.length, 0) +
    fileData.reduce((sum, b) => sum + b.length, 0);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(0, 4);
  eocd.writeUInt16LE(0, 6);
  eocd.writeUInt16LE(files.length, 8);
  eocd.writeUInt16LE(files.length, 10);
  eocd.writeUInt32LE(
    centralEntries.reduce((sum, e) => sum + e.length, 0),
    12,
  );
  eocd.writeUInt32LE(centralStart, 16);
  eocd.writeUInt16LE(0, 20);

  return Buffer.concat([...localHeaders, ...fileData, ...centralEntries, eocd]);
}

const crcTable = new Int32Array(256);
for (let i = 0; i < 256; i++) {
  let c = i;
  for (let j = 0; j < 8; j++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  crcTable[i] = c;
}

function crc32(buf: Buffer): number {
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = crcTable[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

test.describe('ZIP batch upload with real backend', () => {
  let tmpDir: string;

  test.beforeEach(() => {
    tmpDir = mkdtempSync(join(tmpdir(), 'zip-upload-test-'));
  });

  test.afterEach(() => {
    for (const name of ['valid.zip', 'too-many.zip', 'corrupted.zip', 'mixed.zip']) {
      try {
        unlinkSync(join(tmpDir, name));
      } catch {
        // ignore
      }
    }
    try {
      rmdirSync(tmpDir);
    } catch {
      // ignore
    }
  });

  test('upload a valid ZIP file → verify files appear in document list', async ({
    page,
    request,
  }) => {
    const collection = await setupAuthAndCollection(page, request, `ZIP ${Date.now()}`);
    writeFileSync(
      join(tmpDir, 'valid.zip'),
      createZipBuffer([{ name: 'README.md', content: '# Hello' }]),
    );

    await page.goto('/admin');
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 10000,
    });
    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });
    await fileInput(page).setInputFiles(join(tmpDir, 'valid.zip'));

    await expect(page.locator('.dl-item__name').first()).toContainText('README.md', {
      timeout: 30000,
    });
  });

  test('upload a ZIP with >10 files → verify backend rejects it', async ({ page, request }) => {
    const collection = await setupAuthAndCollection(page, request, `ZIP Too Many ${Date.now()}`);
    const files = Array.from({ length: 11 }, (_, i) => ({
      name: `doc-${i}.md`,
      content: `# Document ${i}`,
    }));
    writeFileSync(join(tmpDir, 'too-many.zip'), createZipBuffer(files));

    await page.goto('/admin');
    await setActiveCollection(page, collection.id);
    await page.waitForSelector('.drop-zone', { timeout: 10000 });
    await fileInput(page).setInputFiles(join(tmpDir, 'too-many.zip'));

    await page.waitForFunction(
      () => {
        // biome-ignore lint/suspicious/noExplicitAny: E2E test inspects Pinia state
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        return Boolean(pinia.state.value.documents?.error);
      },
      { timeout: 10000 },
    );
  });
});

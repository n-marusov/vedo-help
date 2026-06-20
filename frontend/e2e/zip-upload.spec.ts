import { mkdtempSync, rmdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { expect, test } from '@playwright/test';
import { VALID_TOKEN, mockCollections, setupAuth } from './helpers';

/**
 * Build a minimal valid ZIP file buffer containing the given files.
 * Constructs the ZIP binary manually (local file headers + central directory + EOCD).
 */
function createZipBuffer(files: { name: string; content: Buffer | string }[]): Buffer {
  const localHeaders: Buffer[] = [];
  const centralEntries: Buffer[] = [];
  let dataOffset = 0;

  for (const file of files) {
    const nameBuf = Buffer.from(file.name);
    const contentBuf = typeof file.content === 'string' ? Buffer.from(file.content) : file.content;
    const crc = crc32(contentBuf);

    // Local file header
    const localHeader = Buffer.alloc(30);
    localHeader.writeUInt32LE(0x04034b50, 0); // signature
    localHeader.writeUInt16LE(20, 4); // version needed
    localHeader.writeUInt16LE(0, 6); // flags
    localHeader.writeUInt16LE(0, 8); // compression method (stored)
    localHeader.writeUInt16LE(0, 10); // mod time
    localHeader.writeUInt16LE(0, 12); // mod date
    localHeader.writeUInt32LE(crc, 14); // crc32
    localHeader.writeUInt32LE(contentBuf.length, 18); // compressed size
    localHeader.writeUInt32LE(contentBuf.length, 22); // uncompressed size
    localHeader.writeUInt16LE(nameBuf.length, 26); // filename length
    localHeader.writeUInt16LE(0, 28); // extra field length

    localHeaders.push(Buffer.concat([localHeader, nameBuf]));

    // Central directory entry
    const centralEntry = Buffer.alloc(46);
    centralEntry.writeUInt32LE(0x02014b50, 0); // signature
    centralEntry.writeUInt16LE(20, 4); // version made by
    centralEntry.writeUInt16LE(20, 6); // version needed
    centralEntry.writeUInt16LE(0, 8); // flags
    centralEntry.writeUInt16LE(0, 10); // compression method
    centralEntry.writeUInt16LE(0, 12); // mod time
    centralEntry.writeUInt16LE(0, 14); // mod date
    centralEntry.writeUInt32LE(crc, 16); // crc32
    centralEntry.writeUInt32LE(contentBuf.length, 20); // compressed size
    centralEntry.writeUInt32LE(contentBuf.length, 24); // uncompressed size
    centralEntry.writeUInt16LE(nameBuf.length, 28); // filename length
    centralEntry.writeUInt16LE(0, 30); // extra field length
    centralEntry.writeUInt16LE(0, 32); // file comment length
    centralEntry.writeUInt16LE(0, 34); // disk number start
    centralEntry.writeUInt16LE(0, 36); // internal file attributes
    centralEntry.writeUInt32LE(0, 38); // external file attributes
    centralEntry.writeUInt32LE(dataOffset, 42); // relative offset of local header

    centralEntries.push(Buffer.concat([centralEntry, nameBuf]));
    dataOffset += 30 + nameBuf.length + contentBuf.length;
  }

  // End of central directory record
  const centralStart =
    Buffer.concat(localHeaders).length +
    Buffer.concat(
      files.map((f) => {
        const contentBuf = typeof f.content === 'string' ? Buffer.from(f.content) : f.content;
        return contentBuf;
      }),
    ).length;

  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0); // signature
  eocd.writeUInt16LE(0, 4); // disk number
  eocd.writeUInt16LE(0, 6); // disk with central directory
  eocd.writeUInt16LE(files.length, 8); // number of entries on this disk
  eocd.writeUInt16LE(files.length, 10); // total entries
  eocd.writeUInt32LE(
    centralEntries.reduce((sum, e) => sum + e.length, 0),
    12,
  ); // size of central directory
  eocd.writeUInt32LE(centralStart, 16); // offset of central directory
  eocd.writeUInt16LE(0, 20); // comment length

  const fileData = files.map((f) => {
    const contentBuf = typeof f.content === 'string' ? Buffer.from(f.content) : f.content;
    return contentBuf;
  });

  return Buffer.concat([...localHeaders, ...fileData, ...centralEntries, eocd]);
}

/** CRC-32 lookup table */
const crcTable = new Int32Array(256);
for (let i = 0; i < 256; i++) {
  let c = i;
  for (let j = 0; j < 8; j++) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
  }
  crcTable[i] = c;
}

function crc32(buf: Buffer): number {
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) {
    crc = crcTable[(crc ^ buf[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

test.describe('ZIP batch upload', () => {
  let tmpDir: string;

  test.beforeEach(async ({ page }) => {
    tmpDir = mkdtempSync(join(tmpdir(), 'zip-upload-test-'));

    // Login via API key + JWT + mock collections
    // DEBUG [e2e] zip-upload: mocking collections + setting API key
    await page.addInitScript((token: string) => {
      localStorage.setItem('vedo_auth_token', token);
    }, VALID_TOKEN);

    // Mock collections so admin panel shows documents area
    await mockCollections(page);

    await page.goto('/');
  });

  test.afterEach(() => {
    // Clean up temp files
    try {
      const files = ['valid.zip', 'too-many.zip', 'corrupted.zip', 'mixed.zip'];
      for (const f of files) {
        try {
          unlinkSync(join(tmpDir, f));
        } catch {
          /* ignore */
        }
      }
      rmdirSync(tmpDir);
    } catch {
      /* ignored */
    }
  });

  test('upload a valid ZIP file → verify files appear in document list', async ({ page }) => {
    const zipBuf = createZipBuffer([{ name: 'README.md', content: '# Hello' }]);
    writeFileSync(join(tmpDir, 'valid.zip'), zipBuf);

    // Mock ZIP upload endpoint (uses XMLHttpRequest)
    await page.route('**/api/documents/upload-zip', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          processed: 1,
          total_files: 1,
          failed: 0,
        }),
      });
    });
    // Mock documents list
    await page.route('**/api/documents*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'doc-1',
            name: 'README.md',
            file_type: 'text/markdown',
            file_size: 10,
            uploaded_at: new Date().toISOString(),
            collection_id: 'col-1',
          },
        ]),
      });
    });

    await page.goto('/admin');

    // Wait for admin view
    await expect(page.locator('[data-testid="admin-view"]')).toBeVisible({
      timeout: 5000,
    });

    // Set active collection to enable DocumentList
    await page.evaluate(() => {
      // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
      const app = (document.querySelector('#app') as any).__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    // Wait for VDropZone to render after setting activeCollectionId
    await page.waitForSelector('.drop-zone');

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(join(tmpDir, 'valid.zip'));

    // Wait for upload to complete
    await page.waitForSelector('.dl-item', { timeout: 15000 });
    const items = page.locator('.dl-item__name');
    await expect(items.first()).toContainText('README.md');
  });

  test('upload a ZIP with >10 files → verify 413 error', async ({ page }) => {
    const files = Array.from({ length: 11 }, (_, i) => ({
      name: `doc-${i}.md`,
      content: `# Document ${i}`,
    }));
    const zipBuf = createZipBuffer(files);
    writeFileSync(join(tmpDir, 'too-many.zip'), zipBuf);

    await page.goto('/admin');

    // Mock ZIP upload to return 413 (too many files)
    let uploadCalled = false;
    await page.route('**/api/documents/upload-zip', async (route) => {
      uploadCalled = true;
      await route.fulfill({
        status: 413,
        contentType: 'application/json',
        body: JSON.stringify({ error: { message: 'Too many files' } }),
      });
    });

    // Set active collection to enable DocumentList
    await page.evaluate(() => {
      // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
      const app = (document.querySelector('#app') as any).__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    // Wait for VDropZone to render after setting activeCollectionId
    await page.waitForSelector('.drop-zone');

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(join(tmpDir, 'too-many.zip'));

    // Verify the upload endpoint was called
    await expect(async () => {
      expect(uploadCalled).toBe(true);
    }).toPass({ timeout: 5000 });

    // Verify error was set in Pinia store
    await page.waitForFunction(
      () => {
        // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        const err = pinia.state.value.documents?.error;
        return err && err.length > 0;
      },
      { timeout: 5000 },
    );
  });

  test('upload a corrupted/invalid ZIP → verify error handling', async ({ page }) => {
    // Write random bytes that aren't a valid ZIP
    writeFileSync(join(tmpDir, 'corrupted.zip'), Buffer.from([0x00, 0x01, 0x02, 0x03, 0x04, 0x05]));

    await page.goto('/admin');

    // Mock ZIP upload to return error for invalid file
    let uploadCalled = false;
    await page.route('**/api/documents/upload-zip', async (route) => {
      uploadCalled = true;
      await route.fulfill({
        status: 400,
        contentType: 'application/json',
        body: JSON.stringify({
          error: { message: 'Invalid ZIP file: not a valid archive' },
        }),
      });
    });

    // Set active collection to enable DocumentList
    await page.evaluate(() => {
      // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
      const app = (document.querySelector('#app') as any).__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    // Wait for VDropZone to render after setting activeCollectionId
    await page.waitForSelector('.drop-zone');

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(join(tmpDir, 'corrupted.zip'));

    // Verify the upload endpoint was called
    await expect(async () => {
      expect(uploadCalled).toBe(true);
    }).toPass({ timeout: 5000 });

    // Verify error was set in Pinia store
    await page.waitForFunction(
      () => {
        // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
        const app = (document.querySelector('#app') as any).__vue_app__;
        const pinia = app.config.globalProperties.$pinia;
        const err = pinia.state.value.documents?.error;
        return err && err.length > 0;
      },
      { timeout: 5000 },
    );
  });

  test('upload a ZIP with mixed supported/unsupported files → verify partial success', async ({
    page,
  }) => {
    const zipBuf = createZipBuffer([
      { name: 'valid.md', content: '# Valid' },
      { name: 'script.exe', content: 'fake exe' },
      { name: 'notes.txt', content: 'Plain text' },
    ]);
    writeFileSync(join(tmpDir, 'mixed.zip'), zipBuf);

    await page.goto('/admin');

    // Mock ZIP upload to return partial success
    await page.route('**/api/documents/upload-zip', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          processed: 2,
          total_files: 3,
          failed: 1,
        }),
      });
    });

    // Mock documents list (after upload)
    await page.route('**/api/documents*', async (route) => {
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify([
          {
            id: 'doc-1',
            name: 'valid.md',
            file_type: 'text/markdown',
            file_size: 10,
            uploaded_at: new Date().toISOString(),
            collection_id: 'col-1',
          },
        ]),
      });
    });

    // Set active collection to enable DocumentList
    await page.evaluate(() => {
      // biome-ignore lint/suspicious/noExplicitAny: Vue internal property
      const app = (document.querySelector('#app') as any).__vue_app__;
      const pinia = app.config.globalProperties.$pinia;
      pinia.state.value.collections.activeCollectionId = 'col-1';
    });

    // Wait for VDropZone to render after setting activeCollectionId
    await page.waitForSelector('.drop-zone');

    const fileInput = page.locator('input[type="file"]');
    await fileInput.setInputFiles(join(tmpDir, 'mixed.zip'));

    // Wait for upload to complete with partial results
    await page.waitForSelector('.dl-item, .dl-result, .zip-result', {
      timeout: 15000,
    });
    await expect(page.locator('.dl-item__name').first()).toBeVisible();
  });
});

import { describe, expect, test } from "vitest";
import client from "./client.ts?raw";
import types from "./types.ts?raw";

describe("metadata archive api client", () => {
  test("download type carries the browser blob and archive headers", () => {
    expect(types).toMatch(/export type MetadataExportDownload = \{/);
    expect(types).toMatch(/blob: Blob;/);
    expect(types).toMatch(/filename: string;/);
    expect(types).toMatch(/files: number \| null;/);
    expect(types).toMatch(/bytes: number \| null;/);
  });

  test("metadataExport posts to the settings-gated download endpoint", () => {
    expect(client).toMatch(
      /metadataExport: async \(\): Promise<MetadataExportDownload> => \{/,
    );
    expect(client).toMatch(
      /fetch\(apiPath\("\/api\/metadata\/export"\), \{[\s\S]{1,160}method: "POST"/,
    );
    expect(client).toMatch(/headers: directAuthHeaders\(\)/);
    expect(client).toMatch(
      /contentDispositionFilename\(res\.headers\.get\("content-disposition"\)\)/,
    );
    expect(client).toMatch(/numericHeader\(res, "x-chan-metadata-files"\)/);
    expect(client).toMatch(/numericHeader\(res, "x-chan-metadata-bytes"\)/);
  });
});

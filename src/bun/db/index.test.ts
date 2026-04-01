import { afterEach, expect, test } from "bun:test";
import { mkdtempSync, rmSync } from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";

import {
	deleteFileRecordByPath,
	getFileRecordByPath,
	openDatabase,
	upsertFileRecord,
} from "./index";

const testDirectories = new Set<string>();

afterEach(() => {
	for (const directory of testDirectories) {
		rmSync(directory, { force: true, recursive: true });
		testDirectories.delete(directory);
	}
});

test("database migrations create the expected tables", () => {
	const directory = mkdtempSync(join(tmpdir(), "nomen-db-schema-"));
	testDirectories.add(directory);

	const database = openDatabase({ path: join(directory, "index.db") });
	const tables = database
		.prepare(
			"SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name ASC;",
		)
		.all() as Array<{ name: string }>;

	expect(tables.map((table) => table.name)).toEqual(
		expect.arrayContaining([
			"columns",
			"files",
			"folder_views",
			"metadata",
			"views",
			"write_queue",
		]),
	);

	database.close();
});

test("file rows can be inserted, queried, and deleted", () => {
	const directory = mkdtempSync(join(tmpdir(), "nomen-db-file-"));
	testDirectories.add(directory);

	const database = openDatabase({ path: join(directory, "index.db") });
	const filePath = join(directory, "plain.txt");

	const fileId = upsertFileRecord(database, {
		path: filePath,
		filename: "plain.txt",
		extension: "txt",
		sizeBytes: 32,
		mtime: 1234,
		inode: 5678,
		fileKind: "document",
	});

	expect(fileId).toBeGreaterThan(0);
	expect(getFileRecordByPath(database, filePath)).toMatchObject({
		filename: "plain.txt",
		fileKind: "document",
		path: filePath,
	});
	expect(deleteFileRecordByPath(database, filePath)).toBe(true);
	expect(getFileRecordByPath(database, filePath)).toBeNull();

	database.close();
});
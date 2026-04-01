import { expect, test } from "bun:test";

import type {
	BulkWrite,
	ColumnDefinition,
	FileOperation,
	FileRow,
	MetadataFieldKey,
	Namespace,
	WriteResult,
} from "./types";

type Assert<T extends true> = T;
type IsExact<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() =>
	T extends B ? 1 : 2
	? true
	: false;

type _NamespaceCoverage = Assert<
	IsExact<
		Namespace,
		"EXIF" | "IPTC" | "XMP" | "ID3" | "xattr" | "user" | "system"
	>
>;

type _MetadataFieldKeyFormat = Assert<
	MetadataFieldKey extends `${Namespace}:${string}` ? true : false
>;

const namespaceCoverageCheck: _NamespaceCoverage = true;
const metadataFieldKeyFormatCheck: _MetadataFieldKeyFormat = true;

void namespaceCoverageCheck;
void metadataFieldKeyFormatCheck;

test("column definitions remain writable and sortable by design", () => {
	const column: ColumnDefinition = {
		label: "Title",
		namespace: "XMP",
		key: "XMP:Title",
		dataType: "text",
		writeDest: "embedded_xmp",
		widthPx: 240,
		isSortable: true,
		isEditable: true,
	};

	expect(column.namespace).toBe("XMP");
	expect(column.writeDest).toBe("embedded_xmp");
});

test("file rows carry fully qualified metadata keys", () => {
	const row: FileRow = {
		id: 1,
		path: "/fixtures/photo.jpg",
		filename: "photo.jpg",
		extension: "jpg",
		sizeBytes: 1024,
		mtime: 1,
		fileKind: "image",
		thumbnailPath: null,
		metadata: {
			"XMP:Title": "Example",
		},
	};

	expect(row.metadata["XMP:Title"]).toBe("Example");
});

test("bulk writes and file ops expose the expected command shapes", () => {
	const write: BulkWrite = {
		fileIds: [1, 2, 3],
		namespace: "IPTC",
		key: "IPTC:CopyrightNotice",
		value: "Example",
	};

	const operation: FileOperation = {
		type: "open",
		paths: ["/fixtures/photo.jpg"],
	};

	const result: WriteResult = {
		success: true,
		affectedFiles: write.fileIds.length,
		failedFiles: 0,
		errors: [],
	};

	expect(write.fileIds).toHaveLength(3);
	expect(operation.type).toBe("open");
	expect(result.success).toBe(true);
});
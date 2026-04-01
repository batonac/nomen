import { mkdirSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join } from "node:path";

import { Database } from "@tursodatabase/database/compat";

import type { DatabaseFileRecord, NewDatabaseFileRecord } from "../../shared/types";
import { SCHEMA_VERSION, schemaMigrations } from "./schema";

export interface OpenDatabaseOptions {
	path?: string;
	inMemory?: boolean;
}

let singleton: Database | null = null;
let singletonPath: string | null = null;

const insertFileStatement = `
	INSERT INTO files (
		path,
		filename,
		extension,
		size_bytes,
		mtime,
		inode,
		content_hash,
		file_kind,
		indexed_at,
		thumbnail_path
	)
	VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	ON CONFLICT(path) DO UPDATE SET
		filename = excluded.filename,
		extension = excluded.extension,
		size_bytes = excluded.size_bytes,
		mtime = excluded.mtime,
		inode = excluded.inode,
		content_hash = excluded.content_hash,
		file_kind = excluded.file_kind,
		indexed_at = excluded.indexed_at,
		thumbnail_path = excluded.thumbnail_path
	RETURNING id;
`;

const selectFileByPathStatement = `
	SELECT
		id,
		path,
		filename,
		extension,
		size_bytes,
		mtime,
		inode,
		content_hash,
		file_kind,
		indexed_at,
		thumbnail_path
	FROM files
	WHERE path = ?;
`;

const deleteFileByPathStatement = "DELETE FROM files WHERE path = ?;";

function now() {
	return Date.now();
}

function coerceNullableNumber(value: unknown): number | null {
	return typeof value === "number" ? value : null;
}

function coerceNullableString(value: unknown): string | null {
	return typeof value === "string" ? value : null;
}

function mapFileRecord(
	row: Record<string, unknown> | null | undefined,
): DatabaseFileRecord | null {
	if (row == null) {
		return null;
	}

	return {
		id: Number(row.id),
		path: String(row.path),
		filename: String(row.filename),
		extension: coerceNullableString(row.extension),
		sizeBytes: coerceNullableNumber(row.size_bytes),
		mtime: Number(row.mtime),
		inode: coerceNullableNumber(row.inode),
		contentHash: coerceNullableString(row.content_hash),
		fileKind: String(row.file_kind) as DatabaseFileRecord["fileKind"],
		indexedAt: Number(row.indexed_at),
		thumbnailPath: coerceNullableString(row.thumbnail_path),
	};
}

export function resolveDatabasePath(customPath?: string): string {
	if (customPath) {
		return customPath;
	}

	if (process.env.NOMEN_DB_PATH) {
		return process.env.NOMEN_DB_PATH;
	}

	return join(homedir(), ".nomen", "index.db");
}

export function openDatabase(options: OpenDatabaseOptions = {}): Database {
	const databasePath = options.inMemory ? ":memory:" : resolveDatabasePath(options.path);

	if (!options.inMemory) {
		mkdirSync(dirname(databasePath), { recursive: true });
	}

	const database = new Database(databasePath, {
		experimental: ["strict"],
	});
	database.exec("PRAGMA foreign_keys = ON;");
	applyMigrations(database);
	return database;
}

export function getDatabase(): Database {
	if (singleton === null) {
		singletonPath = resolveDatabasePath();
		singleton = openDatabase({ path: singletonPath });
	}

	return singleton;
}

export function initializeDatabase(): { path: string; database: Database } {
	const database = getDatabase();
	return {
		path: singletonPath ?? resolveDatabasePath(),
		database,
	};
}

export function closeDatabase(): void {
	if (singleton !== null) {
		singleton.close();
		singleton = null;
		singletonPath = null;
	}
}

export function applyMigrations(database: Database): void {
	const pragmaRow = database.prepare("PRAGMA user_version;").get() as
		| { user_version?: number }
		| undefined;
	const currentVersion = Number(pragmaRow?.user_version ?? 0);

	if (currentVersion >= SCHEMA_VERSION) {
		return;
	}

	database.transaction(() => {
		for (let version = currentVersion + 1; version <= SCHEMA_VERSION; version += 1) {
			const statements = schemaMigrations[version] ?? [];
			for (const statement of statements) {
				database.exec(statement);
			}
		}

		database.exec(`PRAGMA user_version = ${SCHEMA_VERSION};`);
	})();
}

export function upsertFileRecord(
	database: Database,
	fileRecord: NewDatabaseFileRecord,
): number {
	const indexedAt = fileRecord.indexedAt ?? now();
	const statement = database.prepare(insertFileStatement);
	const row = statement.get(
		fileRecord.path,
		fileRecord.filename,
		fileRecord.extension,
		fileRecord.sizeBytes,
		fileRecord.mtime,
		fileRecord.inode,
		fileRecord.contentHash ?? null,
		fileRecord.fileKind,
		indexedAt,
		fileRecord.thumbnailPath ?? null,
	) as { id: number } | null;

	if (row === null) {
		throw new Error(`Failed to upsert file record for ${fileRecord.path}`);
	}

	return Number(row.id);
}

export function getFileRecordByPath(
	database: Database,
	path: string,
): DatabaseFileRecord | null {
	const row = database.prepare(selectFileByPathStatement).get(path) as
		| Record<string, unknown>
		| null;
	return mapFileRecord(row);
}

export function deleteFileRecordByPath(database: Database, path: string): boolean {
	const result = database.prepare(deleteFileByPathStatement).run(path);
	return result.changes > 0;
}
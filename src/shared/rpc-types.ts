import type {
	BulkWrite,
	ColumnDefinition,
	ExifData,
	FileOpResult,
	FileOperation,
	FileRow,
	IndexProgress,
	MetadataRow,
	MetadataWrite,
	NamedView,
	WriteResult,
} from "./types";

/**
 * Tauri command signatures — invoked from the frontend via `invoke()`.
 * The Rust backend implements these as `#[tauri::command]` functions.
 */
export interface TauriCommands {
	navigate_to(path: string): Promise<FileRow[]>;
	get_metadata(fileId: number): Promise<ExifData | MetadataRow[]>;
	write_metadata(writes: MetadataWrite[]): Promise<WriteResult>;
	bulk_write(write: BulkWrite): Promise<WriteResult>;
	file_op(operation: FileOperation): Promise<FileOpResult>;
	get_views(): Promise<NamedView[]>;
	save_view(view: NamedView): Promise<void>;
	add_column(column: ColumnDefinition): Promise<void>;
}

/**
 * Tauri event names — emitted from Rust via `app.emit()`.
 * The frontend listens via `listen()` from `@tauri-apps/api/event`.
 */
export interface TauriEvents {
	"index-update": FileRow[];
	"write-result": WriteResult;
	"index-progress": IndexProgress;
}

export type TauriCommandName = keyof TauriCommands;

export type TauriEventName = keyof TauriEvents;
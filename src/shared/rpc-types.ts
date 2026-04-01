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

export interface WebviewToMainRpc {
	navigateTo(path: string): Promise<FileRow[]>;
	getMetadata(fileId: number): Promise<ExifData | MetadataRow[]>;
	writeMetadata(writes: MetadataWrite[]): Promise<WriteResult>;
	bulkWrite(write: BulkWrite): Promise<WriteResult>;
	fileOp(operation: FileOperation): Promise<FileOpResult>;
	getViews(): Promise<NamedView[]>;
	saveView(view: NamedView): Promise<void>;
	addColumn(column: ColumnDefinition): Promise<void>;
}

export interface MainToWebviewRpc {
	indexUpdate(rows: FileRow[]): void | Promise<void>;
	writeResult(result: WriteResult): void | Promise<void>;
	indexProgress(progress: IndexProgress): void | Promise<void>;
}

export type WebviewToMainMethod = keyof WebviewToMainRpc;

export type MainToWebviewMethod = keyof MainToWebviewRpc;

export type RpcRequestPayload<TMethod extends WebviewToMainMethod> = Parameters<
	WebviewToMainRpc[TMethod]
>[0];

export type RpcResponsePayload<TMethod extends WebviewToMainMethod> = Awaited<
	ReturnType<WebviewToMainRpc[TMethod]>
>;
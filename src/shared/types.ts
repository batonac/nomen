export type FileKind = "image" | "audio" | "video" | "document" | "folder" | "other";

export type Namespace =
	| "EXIF"
	| "IPTC"
	| "XMP"
	| "ID3"
	| "xattr"
	| "user"
	| "system";

export type DataType = "text" | "number" | "date" | "rating" | "tags" | "boolean";

export type WriteDest = "embedded_xmp" | "xmp_sidecar" | "xattr";

export type WriteStatus = "pending" | "complete" | "failed";

export type IndexPhase = "scanning" | "extracting" | "complete";

export type MetadataFieldKey = `${Namespace}:${string}`;

export type MetadataValue = string | null;

export type MetadataMap = Partial<Record<MetadataFieldKey, MetadataValue>>;

export interface FileRow {
	id: number;
	path: string;
	filename: string;
	extension: string | null;
	sizeBytes: number | null;
	mtime: number;
	fileKind: FileKind;
	thumbnailPath: string | null;
	metadata: MetadataMap;
}

export interface MetadataRow {
	id: number;
	fileId: number;
	namespace: Namespace;
	key: string;
	value: MetadataValue;
	dataType: DataType;
	updatedAt: number;
}

export interface ColumnDefinition {
	id?: number;
	label: string;
	namespace: Namespace;
	key: string;
	dataType: DataType;
	writeDest: WriteDest;
	widthPx: number;
	isSortable: boolean;
	isEditable: boolean;
	createdAt?: number;
}

export interface ViewColumnState {
	columnId: number;
	widthPx: number;
	frozen: boolean;
}

export interface NamedView {
	id?: number;
	name: string;
	columns: ViewColumnState[];
	createdAt?: number;
	updatedAt?: number;
}

export interface FolderViewPreference {
	path: string;
	viewId: number;
	applyToChildren: boolean;
}

export interface MetadataWrite {
	fileId: number;
	namespace: Namespace;
	key: string;
	oldValue: MetadataValue;
	newValue: MetadataValue;
}

export interface BulkWrite {
	fileIds: number[];
	namespace: Namespace;
	key: string;
	value: MetadataValue;
}

export interface WriteError {
	fileId: number;
	path: string;
	message: string;
}

export interface WriteResult {
	success: boolean;
	affectedFiles: number;
	failedFiles: number;
	errors: WriteError[];
}

export interface IndexProgress {
	folderPath: string;
	total: number;
	indexed: number;
	phase: IndexPhase;
}

export interface FileOperationOpen {
	type: "open";
	paths: string[];
}

export interface FileOperationReveal {
	type: "reveal";
	paths: string[];
}

export interface FileOperationRename {
	type: "rename";
	path: string;
	nextName: string;
}

export interface FileOperationMove {
	type: "move" | "copy";
	paths: string[];
	destination: string;
}

export interface FileOperationDelete {
	type: "delete";
	paths: string[];
}

export type FileOperation =
	| FileOperationOpen
	| FileOperationReveal
	| FileOperationRename
	| FileOperationMove
	| FileOperationDelete;

export interface FileOpResult {
	success: boolean;
	message?: string;
	paths?: string[];
}

export interface ExifData {
	path: string;
	tags: MetadataMap;
	raw: Record<string, unknown>;
}

export interface DatabaseFileRecord {
	id: number;
	path: string;
	filename: string;
	extension: string | null;
	sizeBytes: number | null;
	mtime: number;
	inode: number | null;
	contentHash: string | null;
	fileKind: FileKind;
	indexedAt: number;
	thumbnailPath: string | null;
}

export interface NewDatabaseFileRecord {
	path: string;
	filename: string;
	extension: string | null;
	sizeBytes: number | null;
	mtime: number;
	inode: number | null;
	contentHash?: string | null;
	fileKind: FileKind;
	indexedAt?: number;
	thumbnailPath?: string | null;
}
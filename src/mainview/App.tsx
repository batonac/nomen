import type { ColumnDefinition, FileRow, IndexProgress } from "../shared/types";

const defaultColumns: ColumnDefinition[] = [
	{
		label: "Name",
		namespace: "system",
		key: "system:filename",
		dataType: "text",
		writeDest: "xattr",
		widthPx: 280,
		isSortable: true,
		isEditable: false,
	},
	{
		label: "Kind",
		namespace: "system",
		key: "system:file_kind",
		dataType: "text",
		writeDest: "xattr",
		widthPx: 132,
		isSortable: true,
		isEditable: false,
	},
	{
		label: "Date Modified",
		namespace: "system",
		key: "system:mtime",
		dataType: "date",
		writeDest: "xattr",
		widthPx: 176,
		isSortable: true,
		isEditable: false,
	},
	{
		label: "Camera Make",
		namespace: "EXIF",
		key: "EXIF:Make",
		dataType: "text",
		writeDest: "embedded_xmp",
		widthPx: 180,
		isSortable: true,
		isEditable: true,
	},
	{
		label: "Title",
		namespace: "XMP",
		key: "XMP:Title",
		dataType: "text",
		writeDest: "embedded_xmp",
		widthPx: 220,
		isSortable: true,
		isEditable: true,
	},
];

const previewRows: FileRow[] = [
	{
		id: 1,
		path: "~/nomen-test-fixtures/photo.jpg",
		filename: "photo.jpg",
		extension: "jpg",
		sizeBytes: 5_734_212,
		mtime: 1_712_860_800_000,
		fileKind: "image",
		thumbnailPath: null,
		metadata: {
			"EXIF:Make": "Canon",
			"XMP:Title": "Cliff path at dusk",
		},
	},
	{
		id: 2,
		path: "~/nomen-test-fixtures/audio.mp3",
		filename: "audio.mp3",
		extension: "mp3",
		sizeBytes: 12_320_554,
		mtime: 1_712_946_020_000,
		fileKind: "audio",
		thumbnailPath: null,
		metadata: {},
	},
	{
		id: 3,
		path: "~/nomen-test-fixtures/document.pdf",
		filename: "document.pdf",
		extension: "pdf",
		sizeBytes: 840_128,
		mtime: 1_713_205_500_000,
		fileKind: "document",
		thumbnailPath: null,
		metadata: {
			"XMP:Title": "Field notes",
		},
	},
	{
		id: 4,
		path: "~/nomen-test-fixtures/subfolder",
		filename: "subfolder",
		extension: null,
		sizeBytes: null,
		mtime: 1_713_246_420_000,
		fileKind: "folder",
		thumbnailPath: null,
		metadata: {},
	},
];

const scannerProgress: IndexProgress = {
	folderPath: "~/nomen-test-fixtures",
	total: 4,
	indexed: 2,
	phase: "extracting",
};

function formatBytes(sizeBytes: number | null) {
	if (sizeBytes === null) {
		return "Folder";
	}

	if (sizeBytes < 1024) {
		return `${sizeBytes} B`;
	}

	const units = ["KB", "MB", "GB", "TB"];
	let value = sizeBytes / 1024;
	let unitIndex = 0;

	while (value >= 1024 && unitIndex < units.length - 1) {
		value /= 1024;
		unitIndex += 1;
	}

	return `${value.toFixed(1)} ${units[unitIndex]}`;
}

function formatTimestamp(timestamp: number) {
	return new Intl.DateTimeFormat(undefined, {
		dateStyle: "medium",
		timeStyle: "short",
	}).format(timestamp);
}

function progressPercent(progress: IndexProgress) {
	if (progress.total === 0) {
		return 0;
	}

	return Math.round((progress.indexed / progress.total) * 100);
}

function App() {
	return (
		<div className="shell">
			<header className="hero">
				<div>
					<p className="eyebrow">Desktop metadata workbench</p>
					<h1>Nomen</h1>
					<p className="lede">
						The foundation layer is in place: shared contracts, a persistent
						local index, and a UI shell aligned with the spec’s grid-first
						workflow.
					</p>
				</div>
				<div className="status-card accent-card">
					<span className="status-label">Indexer</span>
					<strong>{scannerProgress.phase}</strong>
					<div className="progress-track" aria-hidden="true">
						<div
							className="progress-fill"
							style={{ width: `${progressPercent(scannerProgress)}%` }}
						/>
					</div>
					<p>
						{scannerProgress.indexed} of {scannerProgress.total} items indexed in
						 {scannerProgress.folderPath}
					</p>
				</div>
			</header>

			<section className="summary-grid" aria-label="Implementation summary">
				<article className="status-card">
					<span className="status-label">Shared model</span>
					<strong>Typed contracts</strong>
					<p>
						File rows, metadata writes, views, file operations, and RPC surfaces
						now compile from a single shared source.
					</p>
				</article>
				<article className="status-card">
					<span className="status-label">Local index</span>
					<strong>SQLite ready</strong>
					<p>
						The app bootstraps a versioned database at ~/.nomen/index.db and can
						insert, query, and delete file rows.
					</p>
				</article>
				<article className="status-card">
					<span className="status-label">Next milestones</span>
					<strong>Daemon, indexer, RPC</strong>
					<p>
						The remaining work is now unblocked for ExifTool process management,
						background scans, and main/webview request handling.
					</p>
				</article>
			</section>

			<section className="workspace-panel">
				<div className="panel-header">
					<div>
						<p className="eyebrow">Column preview</p>
						<h2>Default workspace</h2>
					</div>
					<button type="button" className="ghost-button">
						+ Add column
					</button>
				</div>

				<div className="column-pills" aria-label="Configured columns">
					{defaultColumns.map((column) => (
						<span key={column.key} className="column-pill">
							{column.label}
						</span>
					))}
				</div>

				<div className="table-wrap">
					<table>
						<thead>
							<tr>
								<th>Name</th>
								<th>Kind</th>
								<th>Size</th>
								<th>Date Modified</th>
								<th>Camera Make</th>
								<th>Title</th>
							</tr>
						</thead>
						<tbody>
							{previewRows.map((row) => (
								<tr key={row.id}>
									<td>
										<div className="name-cell">
											<span className={`kind-badge kind-${row.fileKind}`}>
												{row.fileKind.slice(0, 1).toUpperCase()}
											</span>
											<span>{row.filename}</span>
										</div>
									</td>
									<td>{row.fileKind}</td>
									<td>{formatBytes(row.sizeBytes)}</td>
									<td>{formatTimestamp(row.mtime)}</td>
									<td>{row.metadata["EXIF:Make"] ?? ""}</td>
									<td>{row.metadata["XMP:Title"] ?? ""}</td>
								</tr>
							))}
						</tbody>
					</table>
				</div>
			</section>
		</div>
	);
}

export default App;

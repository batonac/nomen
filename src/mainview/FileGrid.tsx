import {
    DataEditor,
    GridCellKind,
    GridColumnIcon,
    type GridCell,
    type GridColumn,
    type Item,
    type GridSelection,
    type EditableGridCell,
    type Rectangle,
    CompactSelection,
} from "@glideapps/glide-data-grid";
import { useCallback, useRef, useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { FileRow, ColumnDefinition } from "../shared/types";

interface FileGridProps {
    rows: FileRow[];
    onNavigate: (path: string) => void;
    onRowsChange?: (rows: FileRow[]) => void;
}

function formatBytes(sizeBytes: number | null): string {
    if (sizeBytes === null) return "";
    if (sizeBytes < 1024) return `${sizeBytes} B`;
    const units = ["KB", "MB", "GB", "TB"];
    let value = sizeBytes / 1024;
    let unitIndex = 0;
    while (value >= 1024 && unitIndex < units.length - 1) {
        value /= 1024;
        unitIndex += 1;
    }
    return `${value.toFixed(1)} ${units[unitIndex]}`;
}

function formatTimestamp(timestamp: number): string {
    return new Intl.DateTimeFormat(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
    }).format(timestamp);
}

function fileKindIcon(kind: string): string {
    switch (kind) {
        case "folder":
            return "📁";
        case "image":
            return "🖼";
        case "audio":
            return "��";
        case "video":
            return "🎬";
        case "document":
            return "📄";
        default:
            return "📎";
    }
}

// Fixed system columns (always present, left side).
const SYSTEM_COLUMNS: GridColumn[] = [
    { title: "Name", id: "name", width: 300, icon: GridColumnIcon.HeaderString },
    { title: "Kind", id: "kind", width: 80 },
    { title: "Size", id: "size", width: 90 },
    { title: "Modified", id: "mtime", width: 180 },
];

export function FileGrid({ rows, onNavigate, onRowsChange }: FileGridProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const [size, setSize] = useState({ width: 800, height: 600 });
    const [selection, setSelection] = useState<GridSelection>({
        columns: CompactSelection.empty(),
        rows: CompactSelection.empty(),
    });
    const [extraColumns, setExtraColumns] = useState<GridColumn[]>([]);
    const [localRows, setLocalRows] = useState<FileRow[]>(rows);

    // Sync local rows when parent rows change.
    useEffect(() => {
        setLocalRows(rows);
    }, [rows]);

    useEffect(() => {
        const el = containerRef.current;
        if (!el) return;
        const obs = new ResizeObserver((entries) => {
            const { width, height } = entries[0].contentRect;
            setSize({ width, height });
        });
        obs.observe(el);
        return () => obs.disconnect();
    }, []);

    const columns: GridColumn[] = [...SYSTEM_COLUMNS, ...extraColumns];

    const getContent = useCallback(
        (cell: Item): GridCell => {
            const [col, row] = cell;
            const fileRow = localRows[row];
            if (!fileRow) {
                return {
                    kind: GridCellKind.Text,
                    data: "",
                    displayData: "",
                    allowOverlay: false,
                };
            }

            // System columns.
            switch (col) {
                case 0:
                    return {
                        kind: GridCellKind.Text,
                        data: fileRow.filename,
                        displayData: `${fileKindIcon(fileRow.fileKind)} ${fileRow.filename}`,
                        allowOverlay: false,
                    };
                case 1:
                    return {
                        kind: GridCellKind.Text,
                        data: fileRow.fileKind,
                        displayData: fileRow.fileKind,
                        allowOverlay: false,
                    };
                case 2:
                    return {
                        kind: GridCellKind.Text,
                        data: fileRow.sizeBytes?.toString() ?? "",
                        displayData:
                            fileRow.fileKind === "folder"
                                ? "—"
                                : formatBytes(fileRow.sizeBytes),
                        allowOverlay: false,
                    };
                case 3:
                    return {
                        kind: GridCellKind.Text,
                        data: fileRow.mtime.toString(),
                        displayData: formatTimestamp(fileRow.mtime),
                        allowOverlay: false,
                    };
                default: {
                    // Extra metadata column.
                    const colDef = extraColumns[col - SYSTEM_COLUMNS.length];
                    if (!colDef?.id) {
                        return {
                            kind: GridCellKind.Text,
                            data: "",
                            displayData: "",
                            allowOverlay: true,
                        };
                    }
                    const metaKey = `${colDef.id}`;
                    const value = fileRow.metadata[metaKey] ?? "";
                    return {
                        kind: GridCellKind.Text,
                        data: value ?? "",
                        displayData: value ?? "",
                        allowOverlay: true,
                    };
                }
            }
        },
        [localRows, extraColumns]
    );

    const onCellEdited = useCallback(
        async (cell: Item, newValue: EditableGridCell) => {
            const [col, row] = cell;
            if (col < SYSTEM_COLUMNS.length) return; // system columns are read-only

            const fileRow = localRows[row];
            if (!fileRow) return;

            const colDef = extraColumns[col - SYSTEM_COLUMNS.length];
            if (!colDef?.id) return;

            const [namespace, key] = (colDef.id as string).split(":");
            if (!namespace || !key) return;

            const newVal =
                newValue.kind === GridCellKind.Text ? newValue.data : null;
            const oldVal = fileRow.metadata[colDef.id as string] ?? null;

            // Optimistic local update.
            const previousRows = localRows;
            const updated = localRows.map((r, i) =>
                i === row
                    ? {
                          ...r,
                          metadata: {
                              ...r.metadata,
                              [colDef.id as string]: newVal,
                          },
                      }
                    : r
            );
            setLocalRows(updated);
            onRowsChange?.(updated);

            try {
                await invoke("write_metadata", {
                    writes: [
                        {
                            fileId: fileRow.id,
                            namespace,
                            key,
                            oldValue: oldVal,
                            newValue: newVal,
                        },
                    ],
                });
            } catch (e) {
                // Revert on error.
                setLocalRows(previousRows);
                onRowsChange?.(previousRows);
                console.error("write_metadata failed:", e);
            }
        },
        [localRows, extraColumns, onRowsChange]
    );

    // Fill-handle: apply topmost value to all selected rows in same column.
    const onFillPattern = useCallback(
        async (fillRange: Rectangle, patternRange: Rectangle) => {
            if (fillRange.x !== patternRange.x) return;
            const col = fillRange.x;
            if (col < SYSTEM_COLUMNS.length) return;

            const colDef = extraColumns[col - SYSTEM_COLUMNS.length];
            if (!colDef?.id) return;

            const [namespace, key] = (colDef.id as string).split(":");
            if (!namespace || !key) return;

            // Source value: top cell of pattern range.
            const sourceRow = localRows[patternRange.y];
            if (!sourceRow) return;
            const value = sourceRow.metadata[colDef.id as string] ?? null;

            // Collect target file IDs.
            const fileIds: number[] = [];
            for (let r = fillRange.y; r < fillRange.y + fillRange.height; r++) {
                const fr = localRows[r];
                if (fr) fileIds.push(fr.id);
            }

            if (fileIds.length === 0) return;

            // Optimistic local update.
            const previousRows = localRows;
            const updated = localRows.map((r, i) => {
                if (i >= fillRange.y && i < fillRange.y + fillRange.height) {
                    return {
                        ...r,
                        metadata: { ...r.metadata, [colDef.id as string]: value },
                    };
                }
                return r;
            });
            setLocalRows(updated);
            onRowsChange?.(updated);

            try {
                await invoke("bulk_write", {
                    write: { fileIds, namespace, key, value },
                });
            } catch (e) {
                setLocalRows(previousRows);
                onRowsChange?.(previousRows);
                console.error("bulk_write failed:", e);
            }
        },
        [localRows, extraColumns, onRowsChange]
    );

    const onCellActivated = useCallback(
        (cell: Item) => {
            const [_col, row] = cell;
            const fileRow = localRows[row];
            if (fileRow?.fileKind === "folder") {
                onNavigate(fileRow.path);
            } else if (fileRow) {
                invoke("file_op", {
                    operation: { type: "open", paths: [fileRow.path] },
                }).catch(console.error);
            }
        },
        [localRows, onNavigate]
    );

    return (
        <div ref={containerRef} style={{ width: "100%", height: "100%" }}>
            <DataEditor
                getCellContent={getContent}
                columns={columns}
                rows={localRows.length}
                onCellActivated={onCellActivated}
                onCellEdited={onCellEdited}
                onFillPattern={onFillPattern}
                gridSelection={selection}
                onGridSelectionChange={setSelection}
                smoothScrollX
                smoothScrollY
                rowMarkers="clickable-number"
                getCellsForSelection={true}
                fillHandle={true}
                keybindings={{ selectAll: true }}
                width={size.width}
                height={size.height}
            />
        </div>
    );
}

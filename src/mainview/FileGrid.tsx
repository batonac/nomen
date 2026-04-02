import {
    DataEditor,
    GridCellKind,
    GridColumnIcon,
    type GridCell,
    type GridColumn,
    type Item,
} from "@glideapps/glide-data-grid";
import { useCallback, useRef, useState, useEffect } from "react";
import type { FileRow } from "../shared/types";

interface FileGridProps {
    rows: FileRow[];
    onNavigate: (path: string) => void;
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

const columns: GridColumn[] = [
    { title: "Name", id: "name", width: 300, icon: GridColumnIcon.HeaderString },
    { title: "Kind", id: "kind", width: 100 },
    { title: "Size", id: "size", width: 100 },
    { title: "Modified", id: "mtime", width: 180 },
];

export function FileGrid({ rows, onNavigate }: FileGridProps) {
    const containerRef = useRef<HTMLDivElement>(null);
    const [size, setSize] = useState({ width: 800, height: 600 });

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

    const getContent = useCallback(
        (cell: Item): GridCell => {
            const [col, row] = cell;
            const fileRow = rows[row];
            if (!fileRow) {
                return { kind: GridCellKind.Text, data: "", displayData: "", allowOverlay: false };
            }

            switch (col) {
                case 0:
                    return {
                        kind: GridCellKind.Text,
                        data: fileRow.filename,
                        displayData: fileRow.filename,
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
                        displayData: fileRow.fileKind === "folder" ? "—" : formatBytes(fileRow.sizeBytes),
                        allowOverlay: false,
                    };
                case 3:
                    return {
                        kind: GridCellKind.Text,
                        data: fileRow.mtime.toString(),
                        displayData: formatTimestamp(fileRow.mtime),
                        allowOverlay: false,
                    };
                default:
                    return { kind: GridCellKind.Text, data: "", displayData: "", allowOverlay: false };
            }
        },
        [rows]
    );

    const onCellActivated = useCallback(
        (cell: Item) => {
            const [_col, row] = cell;
            const fileRow = rows[row];
            if (fileRow?.fileKind === "folder") {
                onNavigate(fileRow.path);
            }
        },
        [rows, onNavigate]
    );

    return (
        <div ref={containerRef} style={{ width: "100%", height: "100%" }}>
            <DataEditor
                getCellContent={getContent}
                columns={columns}
                rows={rows.length}
                onCellActivated={onCellActivated}
                smoothScrollX
                smoothScrollY
                rowMarkers="none"
                getCellsForSelection={true}
                width={size.width}
                height={size.height}
            />
        </div>
    );
}

export interface PresetColumn {
    label: string;
    namespace: string;
    key: string;
    widthPx?: number;
}

export interface ColumnPreset {
    name: string;
    icon: string;
    columns: PresetColumn[];
}

export const COLUMN_PRESETS: ColumnPreset[] = [
    {
        name: "Images",
        icon: "🖼",
        columns: [
            { label: "Title",        namespace: "XMP",  key: "Title",             widthPx: 160 },
            { label: "Description",  namespace: "XMP",  key: "Description",       widthPx: 220 },
            { label: "Creator",      namespace: "XMP",  key: "Creator",           widthPx: 130 },
            { label: "Keywords",     namespace: "IPTC", key: "Keywords",          widthPx: 200 },
            { label: "Date Taken",   namespace: "EXIF", key: "DateTimeOriginal",  widthPx: 150 },
            { label: "Camera",       namespace: "EXIF", key: "Model",             widthPx: 130 },
            { label: "Aperture",     namespace: "EXIF", key: "FNumber",           widthPx:  70 },
            { label: "Shutter",      namespace: "EXIF", key: "ExposureTime",      widthPx:  90 },
            { label: "ISO",          namespace: "EXIF", key: "ISO",               widthPx:  60 },
            { label: "Focal Length", namespace: "EXIF", key: "FocalLength",       widthPx:  90 },
        ],
    },
    {
        name: "Documents",
        icon: "📄",
        columns: [
            { label: "Title",       namespace: "XMP", key: "Title",       widthPx: 200 },
            { label: "Author",      namespace: "XMP", key: "Creator",     widthPx: 140 },
            { label: "Description", namespace: "XMP", key: "Description", widthPx: 250 },
            { label: "Subject",     namespace: "XMP", key: "Subject",     widthPx: 180 },
            { label: "Keywords",    namespace: "XMP", key: "Subject",     widthPx: 200 },
            { label: "Created",     namespace: "XMP", key: "CreateDate",  widthPx: 140 },
            { label: "Modified",    namespace: "XMP", key: "ModifyDate",  widthPx: 140 },
            { label: "Pages",       namespace: "PDF", key: "PageCount",   widthPx:  60 },
        ],
    },
    {
        name: "Media",
        icon: "🎬",
        columns: [
            { label: "Title",    namespace: "XMP",       key: "Title",           widthPx: 180 },
            { label: "Artist",   namespace: "ID3",       key: "Artist",          widthPx: 140 },
            { label: "Album",    namespace: "ID3",       key: "Album",           widthPx: 150 },
            { label: "Genre",    namespace: "ID3",       key: "Genre",           widthPx: 100 },
            { label: "Year",     namespace: "ID3",       key: "Year",            widthPx:  60 },
            { label: "Track",    namespace: "ID3",       key: "Track",           widthPx:  60 },
            { label: "Duration", namespace: "Composite", key: "Duration",        widthPx:  90 },
            { label: "Bit Rate", namespace: "Audio",     key: "AudioBitrate",    widthPx:  80 },
            { label: "Codec",    namespace: "QuickTime", key: "VideoCodec",      widthPx:  90 },
        ],
    },
];

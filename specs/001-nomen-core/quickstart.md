# Nomen — Quickstart Validation Guide
**Version**: 1.0 | **Date**: 2026-03-31

This document defines the key validation scenarios for confirming Nomen behaves correctly. These scenarios are designed to be run by a human reviewer or an AI agent verifying the implementation against the spec.

---

## Prerequisites

- A folder of test files at `~/nomen-test-fixtures/` containing:
  - `photo.jpg` — a JPEG with EXIF data (camera make, GPS, date taken)
  - `audio.mp3` — an MP3 with ID3 tags (artist, album, track)
  - `document.pdf` — a PDF with XMP metadata (title, author)
  - `plain.txt` — a plain text file with no embedded metadata
  - `subfolder/` — a subfolder containing `nested.jpg`
- Nomen installed and running

---

## Scenario 1 — Basic Navigation (US-01)

**Steps**:
1. Launch Nomen. It opens showing the home folder or last visited folder.
2. Navigate to `~/nomen-test-fixtures/` by typing the path in the breadcrumb bar (press F6, type path, press Enter).

**Expected**:
- Grid shows 5 rows: `photo.jpg`, `audio.mp3`, `document.pdf`, `plain.txt`, `subfolder/`
- Breadcrumb shows the full path with each segment clickable
- `subfolder/` row shows a folder icon in column 0

3. Double-click `subfolder/` in the icon column.

**Expected**:
- Grid updates to show `nested.jpg`
- Breadcrumb updates to show `.../nomen-test-fixtures/subfolder`

4. Press Backspace.

**Expected**:
- Grid returns to `~/nomen-test-fixtures/`

---

## Scenario 2 — Metadata Display (US-02)

**Steps**:
1. Navigate to `~/nomen-test-fixtures/`.
2. Verify default columns are visible: Name, Size, Date Modified, Kind.
3. Click the "+" column button and add column: label "Camera Make", namespace "EXIF", key "EXIF:Make", data type "text".

**Expected**:
- "Camera Make" column appears to the right of existing columns
- `photo.jpg` row shows the camera make value (e.g. "Apple" or "Canon")
- `audio.mp3`, `document.pdf`, `plain.txt` rows show empty "Camera Make" cells
- `subfolder/` row shows an empty "Camera Make" cell

---

## Scenario 3 — Inline Editing (US-04)

**Steps**:
1. Add column: label "Title", namespace "XMP", key "XMP:Title", data type "text".
2. Single-click the "Title" cell for `photo.jpg`.

**Expected**: Row is selected (highlighted). Cell is NOT in edit mode.

3. Double-click the same cell.

**Expected**: Cell enters edit mode. Cursor is inside the text input.

4. Type "Sunset at the Beach". Press Enter.

**Expected**:
- Cell exits edit mode showing "Sunset at the Beach"
- Within 2 seconds, no error notification appears (write-back succeeded)
- Run `exiftool -XMP:Title ~/nomen-test-fixtures/photo.jpg` in a terminal and verify it returns "Sunset at the Beach"

5. Double-click the "Title" cell for `photo.jpg` again. Press Escape.

**Expected**: Cell exits edit mode. Value remains "Sunset at the Beach" (edit cancelled).

---

## Scenario 4 — Bulk Edit (US-05)

**Steps**:
1. Add column: label "Copyright", namespace "IPTC", key "IPTC:CopyrightNotice", data type "text".
2. Click the "Copyright" cell for `photo.jpg`. Type "© 2026 Test User". Press Enter.
3. Single-click the "Copyright" cell for `photo.jpg` to select it.
4. Drag the lower-right fill handle down to cover `audio.mp3` and `document.pdf`.

**Expected**:
- A "Editing 3 files" indicator appears
- After confirmation, all three files show "© 2026 Test User" in the Copyright column
- Verify in terminal: `exiftool -IPTC:CopyrightNotice ~/nomen-test-fixtures/photo.jpg ~/nomen-test-fixtures/audio.mp3 ~/nomen-test-fixtures/document.pdf`

5. Press Cmd+Z / Ctrl+Z.

**Expected**:
- All three files revert to their previous Copyright value (empty or original)
- Terminal verification confirms revert

---

## Scenario 5 — User-Defined Column with xattr (US-08)

**Steps**:
1. Add column: label "Project", namespace "user", key "user:project", data type "text", write destination "xattr".
2. Double-click the "Project" cell for `photo.jpg`. Type "Nomen Demo". Press Enter.

**Expected**:
- Cell shows "Nomen Demo"
- In terminal: `xattr -p user.nomen.project ~/nomen-test-fixtures/photo.jpg` returns "Nomen Demo"

3. Quit and relaunch Nomen. Navigate back to the fixture folder.

**Expected**: `photo.jpg` "Project" cell still shows "Nomen Demo" (persisted from xattr via index rebuild).

---

## Scenario 6 — Named Views (US-06)

**Steps**:
1. Configure columns: Name, Camera Make, Title, Copyright, Project.
2. Click "Save as view". Name it "Photography".
3. Click the view switcher and switch to the default view (Name, Size, Date Modified, Kind).

**Expected**: Grid shows only the four default columns.

4. Switch back to "Photography" view.

**Expected**: Grid shows Name, Camera Make, Title, Copyright, Project — in that order, with saved widths.

---

## Scenario 7 — File Operations (US-09)

**Steps**:
1. Single-click `plain.txt` to select it.
2. Press F2 on the icon column (or right-click → Rename).

**Expected**: Filename cell in column 0 enters edit mode.

3. Type "notes.txt". Press Enter.

**Expected**: File is renamed; grid row updates to show "notes.txt"; filesystem confirms rename.

4. Right-click `notes.txt` → Delete.

**Expected**: Confirmation dialog shows "Delete 1 file: notes.txt?". Confirm.

**Expected**: File is deleted; row disappears from grid; filesystem confirms deletion.

---

## Scenario 8 — Performance (Non-functional)

**Steps**:
1. Create a folder with 10,000 empty JPEG files (use a script: `for i in $(seq 10000); do cp photo.jpg ~/nomen-perf-test/photo_$i.jpg; done`).
2. Navigate to `~/nomen-perf-test/`.

**Expected**: Grid begins rendering rows within 500ms. Scrolling is smooth at 60fps.

3. Edit one cell in the first row.

**Expected**: Cell commits to the index in under 16ms (one frame). No visible lag.

---

## Scenario 9 — ExifTool Daemon Resilience

**Steps**:
1. While Nomen is running and displaying a folder, find the ExifTool process PID and kill it (`kill -9 <pid>`).

**Expected**:
- No crash notification in Nomen
- Within 2 seconds, a new ExifTool process is running (verify via `pgrep exiftool`)
- Background indexing resumes normally
- Any pending cell edits complete successfully

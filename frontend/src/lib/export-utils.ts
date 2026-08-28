import { format } from "date-fns";

type ExportRow = Record<string, string | number | boolean | null | undefined>;

// ─── CSV ──────────────────────────────────────────────────────────────────────

export function generateCSV(
  data: ExportRow[],
  columns: { id: string; label: string }[],
) {
  const headers = columns.map((c) => c.label).join(",");
  const rows = data.map((row) =>
    columns
      .map((col) => {
        const val = row[col.id];
        if (col.id === "date" && val != null) {
          return format(new Date(String(val)), "yyyy-MM-dd HH:mm:ss");
        }
        if (col.id === "success_rate" && typeof val === "number") {
          return (val * 100).toFixed(2) + "%";
        }
        if (typeof val === "string" && val.includes(",")) return `"${val}"`;
        return val ?? "";
      })
      .join(","),
  );

  const csv = [headers, ...rows].join("\n");
  triggerDownload(
    new Blob([csv], { type: "text/csv;charset=utf-8;" }),
    `analytics_export_${format(new Date(), "yyyy-MM-dd")}.csv`,
  );
}

// ─── JSON ─────────────────────────────────────────────────────────────────────

export function generateJSON(
  data: ExportRow[],
  columns: { id: string; label: string }[],
) {
  const filteredData = data.map((row) => {
    const newRow: ExportRow = {};
    columns.forEach((col) => {
      newRow[col.id] = row[col.id];
    });
    return newRow;
  });

  triggerDownload(
    new Blob([JSON.stringify(filteredData, null, 2)], { type: "application/json" }),
    `analytics_export_${format(new Date(), "yyyy-MM-dd")}.json`,
  );
}

// ─── Excel (XLSX via SpreadsheetML) ──────────────────────────────────────────

/**
 * Generates a minimal but valid .xlsx file using the Open XML SpreadsheetML
 * format — no external library required.
 */
export function generateExcel(
  data: ExportRow[],
  columns: { id: string; label: string }[],
  sheetName = "Export",
) {
  const escapeXml = (v: string) =>
    v
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&apos;");

  const cellRef = (col: number, row: number) => {
    let colStr = "";
    let c = col;
    while (c >= 0) {
      colStr = String.fromCharCode(65 + (c % 26)) + colStr;
      c = Math.floor(c / 26) - 1;
    }
    return `${colStr}${row}`;
  };

  const makeRow = (values: string[], rowIdx: number, isHeader = false) =>
    `<row r="${rowIdx}">${values
      .map((v, ci) => {
        const ref = cellRef(ci, rowIdx);
        const safe = escapeXml(v);
        const style = isHeader ? ` s="1"` : "";
        return `<c r="${ref}" t="inlineStr"${style}><is><t>${safe}</t></is></c>`;
      })
      .join("")}</row>`;

  const formatValue = (col: { id: string }, val: ExportRow[string]): string => {
    if (val == null) return "";
    if (col.id === "date") return format(new Date(String(val)), "yyyy-MM-dd HH:mm:ss");
    if (col.id === "success_rate" && typeof val === "number")
      return (val * 100).toFixed(2) + "%";
    return String(val);
  };

  const headerRow = makeRow(columns.map((c) => c.label), 1, true);
  const dataRows = data
    .map((row, i) =>
      makeRow(
        columns.map((col) => formatValue(col, row[col.id])),
        i + 2,
      ),
    )
    .join("");

  const sheetXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <sheetData>${headerRow}${dataRows}</sheetData>
</worksheet>`;

  const stylesXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<styleSheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
  <fonts><font><b/><sz val="11"/></font><font><sz val="11"/></font></fonts>
  <fills><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="gray125"/></fill></fills>
  <borders><border><left/><right/><top/><bottom/><diagonal/></border></borders>
  <cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs>
  <cellXfs>
    <xf numFmtId="0" fontId="1" fillId="0" borderId="0" xfId="0"/>
    <xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"><alignment horizontal="left"/></xf>
  </cellXfs>
</styleSheet>`;

  const workbookXml = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"
          xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
  <sheets><sheet name="${escapeXml(sheetName)}" sheetId="1" r:id="rId1"/></sheets>
</workbook>`;

  const workbookRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
  <Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/>
</Relationships>`;

  const contentTypes = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
  <Default Extension="xml" ContentType="application/xml"/>
  <Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
  <Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
  <Override PartName="/xl/styles.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml"/>
</Types>`;

  const rootRels = `<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
  <Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>`;

  // Build a minimal ZIP (XLSX is a ZIP archive)
  const files: Record<string, string> = {
    "[Content_Types].xml": contentTypes,
    "_rels/.rels": rootRels,
    "xl/workbook.xml": workbookXml,
    "xl/_rels/workbook.xml.rels": workbookRels,
    "xl/worksheets/sheet1.xml": sheetXml,
    "xl/styles.xml": stylesXml,
  };

  const zip = buildZip(files);
  triggerDownload(
    new Blob([zip as BlobPart], { type: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" }),
    `analytics_export_${format(new Date(), "yyyy-MM-dd")}.xlsx`,
  );
}

// ─── PDF ──────────────────────────────────────────────────────────────────────

export async function generatePDF(
  data: ExportRow[],
  columns: { id: string; label: string }[],
  dateRange: { start: Date | null; end: Date | null },
) {
  const [{ default: jsPDF }, { default: autoTable }] = await Promise.all([
    import("jspdf"),
    import("jspdf-autotable"),
  ]);

  const doc = new jsPDF();

  doc.setFontSize(18);
  doc.text("Analytics Export Report", 14, 22);

  doc.setFontSize(11);
  doc.setTextColor(100);
  doc.text(`Generated on: ${format(new Date(), "PPpp")}`, 14, 30);

  if (dateRange.start && dateRange.end) {
    doc.text(
      `Range: ${format(dateRange.start, "yyyy-MM-dd")} to ${format(dateRange.end, "yyyy-MM-dd")}`,
      14,
      36,
    );
  }

  const tableHeaders = columns.map((c) => c.label);
  const tableData = data.map((row) =>
    columns.map((col) => {
      const val = row[col.id];
      if (col.id === "date" && val != null)
        return format(new Date(String(val)), "yyyy-MM-dd HH:mm");
      if (col.id === "success_rate" && typeof val === "number")
        return (val * 100).toFixed(2) + "%";
      if ((col.id === "total_volume" || col.id === "tvl") && typeof val === "number")
        return val.toLocaleString();
      if (col.id === "latency") return `${val} ms`;
      return String(val ?? "");
    }),
  );

  autoTable(doc, {
    head: [tableHeaders],
    body: tableData,
    startY: 44,
    theme: "grid",
    headStyles: { fillColor: [59, 130, 246] },
    styles: { fontSize: 8 },
  });

  doc.save(`analytics_report_${format(new Date(), "yyyy-MM-dd")}.pdf`);
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

function triggerDownload(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.style.visibility = "hidden";
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

/**
 * Minimal ZIP builder — supports only stored (uncompressed) entries, which is
 * sufficient for XLSX since the XML files are small and Excel accepts them.
 */
function buildZip(files: Record<string, string>): Uint8Array {
  const encoder = new TextEncoder();
  const entries: { name: Uint8Array; data: Uint8Array; offset: number }[] = [];
  const parts: Uint8Array[] = [];
  let offset = 0;

  for (const [name, content] of Object.entries(files)) {
    const nameBytes = encoder.encode(name);
    const dataBytes = encoder.encode(content);
    const crc = crc32(dataBytes);

    // Local file header
    const local = new Uint8Array(30 + nameBytes.length);
    const lv = new DataView(local.buffer);
    lv.setUint32(0, 0x04034b50, true); // signature
    lv.setUint16(4, 20, true);          // version needed
    lv.setUint16(6, 0, true);           // flags
    lv.setUint16(8, 0, true);           // compression: stored
    lv.setUint16(10, 0, true);          // mod time
    lv.setUint16(12, 0, true);          // mod date
    lv.setUint32(14, crc, true);        // crc-32
    lv.setUint32(18, dataBytes.length, true); // compressed size
    lv.setUint32(22, dataBytes.length, true); // uncompressed size
    lv.setUint16(26, nameBytes.length, true); // file name length
    lv.setUint16(28, 0, true);          // extra field length
    local.set(nameBytes, 30);

    entries.push({ name: nameBytes, data: dataBytes, offset });
    parts.push(local, dataBytes);
    offset += local.length + dataBytes.length;
  }

  // Central directory
  const cdParts: Uint8Array[] = [];
  let cdSize = 0;
  const cdOffset = offset;

  for (const entry of entries) {
    const crc = crc32(entry.data);
    const cd = new Uint8Array(46 + entry.name.length);
    const cv = new DataView(cd.buffer);
    cv.setUint32(0, 0x02014b50, true);  // signature
    cv.setUint16(4, 20, true);           // version made by
    cv.setUint16(6, 20, true);           // version needed
    cv.setUint16(8, 0, true);            // flags
    cv.setUint16(10, 0, true);           // compression
    cv.setUint16(12, 0, true);           // mod time
    cv.setUint16(14, 0, true);           // mod date
    cv.setUint32(16, crc, true);         // crc-32
    cv.setUint32(20, entry.data.length, true);
    cv.setUint32(24, entry.data.length, true);
    cv.setUint16(28, entry.name.length, true);
    cv.setUint16(30, 0, true);           // extra
    cv.setUint16(32, 0, true);           // comment
    cv.setUint16(34, 0, true);           // disk start
    cv.setUint16(36, 0, true);           // internal attr
    cv.setUint32(38, 0, true);           // external attr
    cv.setUint32(42, entry.offset, true); // local header offset
    cd.set(entry.name, 46);
    cdParts.push(cd);
    cdSize += cd.length;
  }

  // End of central directory
  const eocd = new Uint8Array(22);
  const ev = new DataView(eocd.buffer);
  ev.setUint32(0, 0x06054b50, true);
  ev.setUint16(4, 0, true);
  ev.setUint16(6, 0, true);
  ev.setUint16(8, entries.length, true);
  ev.setUint16(10, entries.length, true);
  ev.setUint32(12, cdSize, true);
  ev.setUint32(16, cdOffset, true);
  ev.setUint16(20, 0, true);

  const all = [...parts, ...cdParts, eocd];
  const total = all.reduce((s, a) => s + a.length, 0);
  const out = new Uint8Array(total);
  let pos = 0;
  for (const a of all) { out.set(a, pos); pos += a.length; }
  return out;
}

/** CRC-32 implementation (IEEE polynomial) */
function crc32(data: Uint8Array): number {
  const table = makeCrcTable();
  let crc = 0xffffffff;
  for (let i = 0; i < data.length; i++) {
    crc = (crc >>> 8) ^ table[(crc ^ data[i]) & 0xff];
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function makeCrcTable(): Uint32Array {
  const t = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let j = 0; j < 8; j++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[i] = c;
  }
  return t;
}

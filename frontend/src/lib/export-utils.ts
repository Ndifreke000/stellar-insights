import jsPDF from "jspdf";
import autoTable from "jspdf-autotable";
import { format } from "date-fns";

type ExportRow = Record<string, string | number | boolean | null | undefined>;

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
  const blob = new Blob([csv], { type: "text/csv;charset=utf-8;" });
  const link = document.createElement("a");
  if (link.download !== undefined) {
    const url = URL.createObjectURL(blob);
    link.setAttribute("href", url);
    link.setAttribute(
      "download",
      `analytics_export_${format(new Date(), "yyyy-MM-dd")}.csv`,
    );
    link.style.visibility = "hidden";
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  }
}

export function generateJSON(
  data: ExportRow[],
  columns: { id: string; label: string }[],
) {
  // Filter data to only include selected columns
  const filteredData = data.map((row) => {
    const newRow: ExportRow = {};
    columns.forEach((col) => {
      newRow[col.id] = row[col.id]; // keep raw values for JSON
    });
    return newRow;
  });

  const json = JSON.stringify(filteredData, null, 2);
  const blob = new Blob([json], { type: "application/json" });
  const link = document.createElement("a");
  if (link.download !== undefined) {
    const url = URL.createObjectURL(blob);
    link.setAttribute("href", url);
    link.setAttribute(
      "download",
      `analytics_export_${format(new Date(), "yyyy-MM-dd")}.json`,
    );
    link.style.visibility = "hidden";
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
  }
}

export function generatePDF(
  data: ExportRow[],
  columns: { id: string; label: string }[],
  dateRange: { start: Date | null; end: Date | null },
) {
  const doc = new jsPDF();

  // Header
  doc.setFontSize(18);
  doc.text("Analytics Export Report", 14, 22);

  doc.setFontSize(11);
  doc.setTextColor(100);
  const dateStr = `Generated on: ${format(new Date(), "PPpp")}`;
  doc.text(dateStr, 14, 30);

  if (dateRange.start && dateRange.end) {
    doc.text(
      `Range: ${format(dateRange.start, "yyyy-MM-dd")} to ${format(dateRange.end, "yyyy-MM-dd")}`,
      14,
      36,
    );
  }

  // Table
  const tableHeaders = columns.map((c) => c.label);
  const tableData = data.map((row) =>
    columns.map((col) => {
      const val = row[col.id];
      if (col.id === "date" && val != null) {
        return format(new Date(String(val)), "yyyy-MM-dd HH:mm");
      }
      if (col.id === "success_rate" && typeof val === "number") {
        return (val * 100).toFixed(2) + "%";
      }
      if ((col.id === "total_volume" || col.id === "tvl") && typeof val === "number") {
        return `$${val.toLocaleString()}`;
      }
      if (col.id === "latency") {
        return `${val} ms`;
      }
      return String(val ?? "");
    }),
  );

  autoTable(doc, {
    head: [tableHeaders],
    body: tableData,
    startY: 44,
    theme: "grid",
    headStyles: { fillColor: [59, 130, 246] }, // Blue-500
    styles: { fontSize: 8 },
  });

  doc.save(`analytics_report_${format(new Date(), "yyyy-MM-dd")}.pdf`);
}

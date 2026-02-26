import { useRef, useCallback, useState } from 'react';
import { exportChart, ExportFormat } from '@/lib/chart-export';

interface UseChartExportOptions {
  chartName: string;
}

export function useChartExport({ chartName }: UseChartExportOptions) {
  const chartRef = useRef<HTMLDivElement>(null);
  const [isExporting, setIsExporting] = useState(false);

  const handleExport = useCallback(
    async (format: ExportFormat) => {
      if (!chartRef.current || isExporting) return;

      setIsExporting(true);

      try {
        const filename = `${chartName.toLowerCase().replace(/\s+/g, '-')}-${new Date().toISOString().split('T')[0]}`;
        await exportChart(chartRef.current, { filename, format });
      } catch (error) {
        console.error('Chart export failed:', error);
        throw error;
      } finally {
        setIsExporting(false);
      }
    },
    [chartName, isExporting]
  );

  return {
    chartRef,
    isExporting,
    handleExport,
  };
}

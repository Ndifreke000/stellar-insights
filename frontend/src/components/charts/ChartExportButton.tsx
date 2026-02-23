'use client';

import { useState, useRef } from 'react';
import { Download, Image, FileImage } from 'lucide-react';
import { exportChart, ExportFormat } from '@/lib/chart-export';

interface ChartExportButtonProps {
  chartRef: React.RefObject<HTMLDivElement>;
  chartName: string;
  className?: string;
}

export function ChartExportButton({
  chartRef,
  chartName,
  className = '',
}: ChartExportButtonProps) {
  const [isExporting, setIsExporting] = useState(false);
  const [showMenu, setShowMenu] = useState(false);

  const handleExport = async (format: ExportFormat) => {
    if (!chartRef.current || isExporting) return;

    setIsExporting(true);
    setShowMenu(false);

    try {
      const filename = `${chartName.toLowerCase().replace(/\s+/g, '-')}-${new Date().toISOString().split('T')[0]}`;
      await exportChart(chartRef.current, { filename, format });
    } catch (error) {
      console.error('Export failed:', error);
    } finally {
      setIsExporting(false);
    }
  };

  return (
    <div className="relative">
      <button
        onClick={() => setShowMenu(!showMenu)}
        disabled={isExporting}
        className={`
          flex items-center gap-2 px-3 py-1.5 
          text-[10px] font-mono uppercase tracking-wider
          bg-slate-900/50 hover:bg-slate-800/50
          border border-white/10 hover:border-accent/50
          rounded-lg transition-all duration-200
          disabled:opacity-50 disabled:cursor-not-allowed
          ${className}
        `}
        aria-label="Export chart"
        aria-expanded={showMenu}
        aria-haspopup="true"
      >
        <Download className="w-3 h-3" />
        {isExporting ? 'Exporting...' : 'Export'}
      </button>

      {showMenu && (
        <>
          <div
            className="fixed inset-0 z-10"
            onClick={() => setShowMenu(false)}
            aria-hidden="true"
          />
          <div
            className="absolute right-0 top-full mt-2 z-20
              bg-slate-900 border border-white/10 rounded-lg
              shadow-xl overflow-hidden min-w-[140px]"
            role="menu"
          >
            <button
              onClick={() => handleExport('png')}
              className="w-full flex items-center gap-2 px-4 py-2.5
                text-[10px] font-mono uppercase tracking-wider
                hover:bg-slate-800 transition-colors text-left"
              role="menuitem"
            >
              <Image className="w-3 h-3 text-accent" />
              PNG Image
            </button>
            <button
              onClick={() => handleExport('svg')}
              className="w-full flex items-center gap-2 px-4 py-2.5
                text-[10px] font-mono uppercase tracking-wider
                hover:bg-slate-800 transition-colors text-left
                border-t border-white/5"
              role="menuitem"
            >
              <FileImage className="w-3 h-3 text-accent" />
              SVG Vector
            </button>
          </div>
        </>
      )}
    </div>
  );
}

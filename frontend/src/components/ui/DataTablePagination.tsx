import React from 'react';
import {
    ChevronLeft,
    ChevronRight,
    ChevronsLeft,
    ChevronsRight
} from 'lucide-react';

interface PaginationProps {
    totalItems: number;
    pageSize: number;
    currentPage: number;
    onPageChange: (page: number) => void;
    onPageSizeChange: (size: number) => void;
}

export function DataTablePagination({
    totalItems,
    pageSize,
    currentPage,
    onPageChange,
    onPageSizeChange,
}: PaginationProps) {
    const totalPages = Math.max(1, Math.ceil(totalItems / pageSize));

    const pageSizes = [10, 25, 50, 100];

    const handleJumpToPage = (e: React.ChangeEvent<HTMLInputElement>) => {
        const value = parseInt(e.target.value);
        if (!isNaN(value)) {
            if (value < 1) onPageChange(1);
            else if (value > totalPages) onPageChange(totalPages);
            else onPageChange(value);
        }
    };

    return (
        <div className="flex flex-col sm:flex-row items-center justify-between gap-4 px-4 py-4 bg-slate-900/50 border-t border-slate-800">
            <div className="text-sm text-slate-400">
                Total <span className="font-medium text-white">{totalItems}</span> records
            </div>

            <div className="flex flex-wrap items-center gap-4 sm:gap-6">
                {/* Page Size Selector */}
                <div className="flex items-center gap-2">
                    <label htmlFor="pageSize" className="text-sm text-slate-400">Rows per page</label>
                    <select
                        id="pageSize"
                        value={pageSize}
                        onChange={(e) => onPageSizeChange(Number(e.target.value))}
                        className="bg-slate-800 border border-slate-700 text-white text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-1.5"
                    >
                        {pageSizes.map((size) => (
                            <option key={size} value={size}>
                                {size}
                            </option>
                        ))}
                    </select>
                </div>

                {/* Jump to Page */}
                <div className="flex items-center gap-2">
                    <label htmlFor="jumpToPage" className="text-sm text-slate-400">Jump to</label>
                    <input
                        id="jumpToPage"
                        type="number"
                        min={1}
                        max={totalPages}
                        value={currentPage}
                        onChange={handleJumpToPage}
                        className="w-16 bg-slate-800 border border-slate-700 text-white text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-1.5"
                    />
                    <span className="text-sm text-slate-400">of {totalPages}</span>
                </div>

                {/* Navigation Buttons */}
                <div className="flex items-center gap-1">
                    <button
                        onClick={() => onPageChange(1)}
                        disabled={currentPage === 1}
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                        title="First Page"
                    >
                        <ChevronsLeft className="w-5 h-5" />
                    </button>
                    <button
                        onClick={() => onPageChange(currentPage - 1)}
                        disabled={currentPage === 1}
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                        title="Previous Page"
                    >
                        <ChevronLeft className="w-5 h-5" />
                    </button>

                    <div className="px-3 py-1.5 text-sm font-medium text-white bg-blue-600 rounded-md">
                        {currentPage}
                    </div>

                    <button
                        onClick={() => onPageChange(currentPage + 1)}
                        disabled={currentPage === totalPages}
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                        title="Next Page"
                    >
                        <ChevronRight className="w-5 h-5" />
                    </button>
                    <button
                        onClick={() => onPageChange(totalPages)}
                        disabled={currentPage === totalPages}
                        className="p-1.5 rounded-md hover:bg-slate-800 disabled:opacity-50 disabled:cursor-not-allowed text-slate-400 hover:text-white transition-colors"
                        title="Last Page"
                    >
                        <ChevronsRight className="w-5 h-5" />
                    </button>
                </div>
            </div>
        </div>
    );
}

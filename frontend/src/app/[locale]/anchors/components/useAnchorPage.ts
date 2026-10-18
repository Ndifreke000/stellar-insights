import { useState, useMemo } from 'react';
import { useRouter } from 'next/navigation';
import { usePagination } from '@/hooks/usePagination';
import { useAnchors } from '@/lib/react-query/queries';

// The page filters/sorts client-side, so request one large page.
const ANCHOR_LIST_PARAMS = { limit: 200 };

const useAnchorPage = () => {
  const router = useRouter();
  const anchorsQuery = useAnchors(ANCHOR_LIST_PARAMS);
  const anchors = useMemo(() => anchorsQuery.data?.data ?? [], [anchorsQuery.data]);
  const loading = anchorsQuery.isPending;
  const error = anchorsQuery.error?.message ?? null;
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<
    'reliability' | 'transactions' | 'failure_rate'
  >('reliability');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [isExportOpen, setIsExportOpen] = useState(false);

  // Filter anchors based on search
  const filteredAnchors = useMemo(() => {
    return anchors.filter(
      (anchor) =>
        anchor.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
        anchor.stellar_account.toLowerCase().includes(searchTerm.toLowerCase()),
    );
  }, [anchors, searchTerm]);

  // Sort and paginate anchors
  const sortedAndFilteredAnchors = useMemo(() => {
    return [...filteredAnchors].sort((a, b) => {
      let comparison = 0;
      switch (sortBy) {
        case 'reliability':
          comparison = b.reliability_score - a.reliability_score;
          break;
        case 'transactions':
          comparison = b.total_transactions - a.total_transactions;
          break;
        case 'failure_rate':
          comparison = a.failure_rate - b.failure_rate;
          break;
        default:
          return 0;
      }
      return sortOrder === 'asc' ? -comparison : comparison;
    });
  }, [filteredAnchors, sortBy, sortOrder]);

  // Pagination
  const {
    currentPage,
    pageSize,
    onPageChange,
    onPageSizeChange,
    startIndex,
    endIndex,
  } = usePagination(sortedAndFilteredAnchors.length);

  const paginatedAnchors = useMemo(() => {
    return sortedAndFilteredAnchors.slice(startIndex, endIndex);
  }, [sortedAndFilteredAnchors, startIndex, endIndex]);
  return {
    router, 
    anchors, 
    loading, 
    error, 
    searchTerm, 
    setSearchTerm, 
    sortBy, 
    setSortBy, 
    sortOrder, 
    setSortOrder, 
    isExportOpen, 
    setIsExportOpen, 
    currentPage, 
    pageSize, 
    onPageChange, 
    onPageSizeChange, 
    paginatedAnchors,sortedAndFilteredAnchors
  };
};

export default useAnchorPage;

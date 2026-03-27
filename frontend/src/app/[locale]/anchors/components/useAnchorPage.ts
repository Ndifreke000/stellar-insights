import { logger } from '@/lib/logger';
import { useState, useEffect, useMemo } from 'react';
import { useRouter } from 'next/navigation';
import { usePagination } from '@/hooks/usePagination';
import { AnchorMetrics } from '@/lib/api/types';
import { fetchAnchors } from '@/lib/api/anchor';

const useAnchorPage = () => {
  const router = useRouter();
  const [anchors, setAnchors] = useState<AnchorMetrics[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [searchTerm, setSearchTerm] = useState('');
  const [sortBy, setSortBy] = useState<
    'reliability' | 'transactions' | 'failure_rate'
  >('reliability');
  const [sortOrder, setSortOrder] = useState<'asc' | 'desc'>('desc');
  const [isExportOpen, setIsExportOpen] = useState(false);

  // Fetch anchors from the backend
  useEffect(() => {
    const loadAnchors = async () => {
      try {
        setLoading(true);
        setError(null);

        // Fetch data from the backend API
        const response = await fetchAnchors({ limit: 100, offset: 0 });
        setAnchors(response.anchors);
      } catch (err) {
        logger.error('Failed to fetch anchors:', err);
        setError(err instanceof Error ? err.message : 'Failed to load anchors');
      } finally {
        setLoading(false);
      }
    };

    loadAnchors();
  }, []);

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

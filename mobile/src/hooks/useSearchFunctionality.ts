import React from 'react';
import { Platform } from 'react-native';
import { Corridor, Anchor, Asset } from '@app-types/index';

export type SearchableItem = Corridor | Anchor | Asset | Record<string, any>;

export interface SearchFilter {
  field?: string;
  value: string | number | boolean;
  operator?: 'equals' | 'contains' | 'startsWith' | 'greaterThan' | 'lessThan';
}

export interface SearchConfig {
  debounceMs?: number;
  minChars?: number;
  maxResults?: number;
  caseSensitive?: boolean;
  fuzzySearch?: boolean;
}

export interface SearchResult<T = SearchableItem> {
  item: T;
  score: number;
  matchedFields: string[];
}

export interface UseSearchFunctionalityResult<T = SearchableItem> {
  query: string;
  results: SearchResult<T>[];
  isSearching: boolean;
  error?: string;
  totalResults: number;
  hasMore: boolean;
  filters: SearchFilter[];
  setQuery: (query: string) => void;
  addFilter: (filter: SearchFilter) => void;
  removeFilter: (index: number) => void;
  clearFilters: () => void;
  clearSearch: () => void;
  loadMore: () => void;
}

const DEFAULT_CONFIG: Required<SearchConfig> = {
  debounceMs: 300,
  minChars: 1,
  maxResults: 50,
  caseSensitive: false,
  fuzzySearch: true,
};

/**
 * Calculates similarity score between query and text (0-1)
 * Simple Levenshtein-based scoring
 */
function calculateSimilarity(query: string, text: string): number {
  const q = query.toLowerCase();
  const t = text.toLowerCase();

  if (q === t) return 1;
  if (t.startsWith(q)) return 0.9;
  if (t.includes(q)) return 0.7;

  // Fuzzy matching
  let score = 0;
  let qIndex = 0;
  for (let i = 0; i < t.length && qIndex < q.length; i++) {
    if (t[i] === q[qIndex]) {
      score += 1;
      qIndex++;
    }
  }

  return qIndex === q.length ? score / (t.length + q.length) : 0;
}

/**
 * Filters items based on search query and filters
 */
function performSearch<T extends SearchableItem>(
  items: T[],
  query: string,
  filters: SearchFilter[],
  config: SearchConfig
): SearchResult<T>[] {
  let results: SearchResult<T>[] = [];

  for (const item of items) {
    let matchedFields: string[] = [];
    let totalScore = 0;

    // Search in all string fields
    for (const [key, value] of Object.entries(item)) {
      if (typeof value === 'string' || typeof value === 'number') {
        const stringValue = String(value);
        const searchText = config.caseSensitive ? stringValue : stringValue.toLowerCase();
        const searchQuery = config.caseSensitive ? query : query.toLowerCase();

        let fieldScore = 0;

        if (searchQuery.length > 0) {
          if (searchText === searchQuery) {
            fieldScore = 1;
          } else if (searchText.startsWith(searchQuery)) {
            fieldScore = 0.9;
          } else if (searchText.includes(searchQuery)) {
            fieldScore = 0.7;
          } else if (config.fuzzySearch) {
            fieldScore = calculateSimilarity(searchQuery, searchText);
          }

          if (fieldScore > 0) {
            matchedFields.push(key);
            totalScore += fieldScore;
          }
        }
      }
    }

    // Apply filters
    let passesFilters = true;
    for (const filter of filters) {
      // item is a union of specific shapes (Corridor | Anchor | Asset) plus a
      // generic Record fallback; filtering is inherently dynamic-field access
      // across that union, so narrow to Record<string, unknown> at the access
      // point rather than losing type safety on the whole function.
      const fieldValue = (item as Record<string, unknown>)[filter.field || ''];
      const operator = filter.operator || 'equals';

      switch (operator) {
        case 'equals':
          if (fieldValue !== filter.value) passesFilters = false;
          break;
        case 'contains':
          if (!String(fieldValue).includes(String(filter.value))) passesFilters = false;
          break;
        case 'startsWith':
          if (!String(fieldValue).startsWith(String(filter.value))) passesFilters = false;
          break;
        case 'greaterThan':
          if (!(Number(fieldValue) > Number(filter.value))) passesFilters = false;
          break;
        case 'lessThan':
          if (!(Number(fieldValue) < Number(filter.value))) passesFilters = false;
          break;
      }

      if (!passesFilters) break;
    }

    if (passesFilters && totalScore > 0) {
      results.push({
        item,
        score: totalScore,
        matchedFields,
      });
    }
  }

  // Sort by score
  results.sort((a, b) => b.score - a.score);
  return results.slice(0, config.maxResults);
}

/**
 * Hook for search functionality with filtering and pagination support
 */
export function useSearchFunctionality<T extends SearchableItem = SearchableItem>(
  items: T[],
  config?: SearchConfig
): UseSearchFunctionalityResult<T> {
  const mergedConfig = { ...DEFAULT_CONFIG, ...config, minChars: config?.minChars ?? DEFAULT_CONFIG.minChars };
  const [query, setQueryState] = React.useState('');
  const [filters, setFilters] = React.useState<SearchFilter[]>([]);
  const [isSearching, setIsSearching] = React.useState(false);
  const [error, setError] = React.useState<string>();
  const [displayedResults, setDisplayedResults] = React.useState<SearchResult<T>[]>([]);
  const [allResults, setAllResults] = React.useState<SearchResult<T>[]>([]);
  const [currentPage, setCurrentPage] = React.useState(0);

  // Debounce search
  const debounceTimer = React.useRef<NodeJS.Timeout>();

  const performSearchOperation = React.useCallback(() => {
    if (query.length < mergedConfig.minChars) {
      setAllResults([]);
      setDisplayedResults([]);
      setCurrentPage(0);
      return;
    }

    setIsSearching(true);
    setError(undefined);

    try {
      const results = performSearch(items, query, filters, mergedConfig);
      setAllResults(results);

      // Display first page
      const pageSize = mergedConfig.maxResults || 50;
      setDisplayedResults(results.slice(0, pageSize));
      setCurrentPage(0);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Search failed';
      setError(message);
    } finally {
      setIsSearching(false);
    }
  }, [query, filters, items, mergedConfig]);

  const setQuery = React.useCallback(
    (newQuery: string) => {
      setQueryState(newQuery);

      // Clear previous timer
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
      }

      // Set new timer
      debounceTimer.current = setTimeout(() => {
        performSearchOperation();
      }, mergedConfig.debounceMs);
    },
    [performSearchOperation, mergedConfig.debounceMs]
  );

  const addFilter = React.useCallback(
    (filter: SearchFilter) => {
      setFilters(prev => [...prev, filter]);
      // Re-run search with new filters
      setTimeout(performSearchOperation, 0);
    },
    [performSearchOperation]
  );

  const removeFilter = React.useCallback(
    (index: number) => {
      setFilters(prev => prev.filter((_, i) => i !== index));
      setTimeout(performSearchOperation, 0);
    },
    [performSearchOperation]
  );

  const clearFilters = React.useCallback(() => {
    setFilters([]);
    setTimeout(performSearchOperation, 0);
  }, [performSearchOperation]);

  const clearSearch = React.useCallback(() => {
    setQueryState('');
    setAllResults([]);
    setDisplayedResults([]);
    setError(undefined);
    setCurrentPage(0);
  }, []);

  const loadMore = React.useCallback(() => {
    const pageSize = mergedConfig.maxResults || 50;
    const nextPage = currentPage + 1;
    const startIndex = nextPage * pageSize;
    const endIndex = startIndex + pageSize;

    if (startIndex < allResults.length) {
      setDisplayedResults(allResults.slice(0, endIndex));
      setCurrentPage(nextPage);
    }
  }, [currentPage, allResults, mergedConfig.maxResults]);

  // Cleanup timer on unmount
  React.useEffect(() => {
    return () => {
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
      }
    };
  }, []);

  const hasMore = (currentPage + 1) * (mergedConfig.maxResults || 50) < allResults.length;

  return {
    query,
    results: displayedResults,
    isSearching,
    error,
    totalResults: allResults.length,
    hasMore,
    filters,
    setQuery,
    addFilter,
    removeFilter,
    clearFilters,
    clearSearch,
    loadMore,
  };
}

/**
 * Advanced search hook with field-specific search capabilities
 */
export function useAdvancedSearch<T extends SearchableItem = SearchableItem>(
  items: T[],
  searchFields: (keyof T)[],
  config?: SearchConfig
): UseSearchFunctionalityResult<T> {
  const mergedConfig = { ...DEFAULT_CONFIG, ...config, minChars: config?.minChars ?? DEFAULT_CONFIG.minChars };
  const [query, setQueryState] = React.useState('');
  const [filters, setFilters] = React.useState<SearchFilter[]>([]);
  const [isSearching, setIsSearching] = React.useState(false);
  const [error, setError] = React.useState<string>();
  const [displayedResults, setDisplayedResults] = React.useState<SearchResult<T>[]>([]);
  const [allResults, setAllResults] = React.useState<SearchResult<T>[]>([]);
  const [currentPage, setCurrentPage] = React.useState(0);

  const debounceTimer = React.useRef<NodeJS.Timeout>();

  const performSearchOperation = React.useCallback(() => {
    if (query.length < mergedConfig.minChars) {
      setAllResults([]);
      setDisplayedResults([]);
      setCurrentPage(0);
      return;
    }

    setIsSearching(true);
    setError(undefined);

    try {
      const results: SearchResult<T>[] = [];

      for (const item of items) {
        let matchedFields: string[] = [];
        let totalScore = 0;

        // Search only in specified fields
        for (const field of searchFields) {
          const value = item[field];
          if (typeof value === 'string' || typeof value === 'number') {
            const stringValue = String(value);
            const searchText = mergedConfig.caseSensitive ? stringValue : stringValue.toLowerCase();
            const searchQuery = mergedConfig.caseSensitive ? query : query.toLowerCase();

            let fieldScore = 0;

            if (searchText === searchQuery) {
              fieldScore = 1;
            } else if (searchText.startsWith(searchQuery)) {
              fieldScore = 0.9;
            } else if (searchText.includes(searchQuery)) {
              fieldScore = 0.7;
            } else if (mergedConfig.fuzzySearch) {
              fieldScore = calculateSimilarity(searchQuery, stringValue);
            }

            if (fieldScore > 0) {
              matchedFields.push(String(field));
              totalScore += fieldScore;
            }
          }
        }

        // Apply filters
        let passesFilters = true;
        for (const filter of filters) {
          const fieldValue = item[filter.field as keyof T];
          const operator = filter.operator || 'equals';

          switch (operator) {
            case 'equals':
              if (fieldValue !== filter.value) passesFilters = false;
              break;
            case 'contains':
              if (!String(fieldValue).includes(String(filter.value))) passesFilters = false;
              break;
            case 'startsWith':
              if (!String(fieldValue).startsWith(String(filter.value))) passesFilters = false;
              break;
            case 'greaterThan':
              if (!(Number(fieldValue) > Number(filter.value))) passesFilters = false;
              break;
            case 'lessThan':
              if (!(Number(fieldValue) < Number(filter.value))) passesFilters = false;
              break;
          }

          if (!passesFilters) break;
        }

        if (passesFilters && totalScore > 0) {
          results.push({
            item,
            score: totalScore,
            matchedFields,
          });
        }
      }

      // Sort by score
      results.sort((a, b) => b.score - a.score);
      setAllResults(results);

      // Display first page
      const pageSize = mergedConfig.maxResults || 50;
      setDisplayedResults(results.slice(0, pageSize));
      setCurrentPage(0);
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Advanced search failed';
      setError(message);
    } finally {
      setIsSearching(false);
    }
  }, [query, filters, items, searchFields, mergedConfig]);

  const setQuery = React.useCallback(
    (newQuery: string) => {
      setQueryState(newQuery);

      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
      }

      debounceTimer.current = setTimeout(() => {
        performSearchOperation();
      }, mergedConfig.debounceMs);
    },
    [performSearchOperation, mergedConfig.debounceMs]
  );

  const addFilter = React.useCallback(
    (filter: SearchFilter) => {
      setFilters(prev => [...prev, filter]);
      setTimeout(performSearchOperation, 0);
    },
    [performSearchOperation]
  );

  const removeFilter = React.useCallback(
    (index: number) => {
      setFilters(prev => prev.filter((_, i) => i !== index));
      setTimeout(performSearchOperation, 0);
    },
    [performSearchOperation]
  );

  const clearFilters = React.useCallback(() => {
    setFilters([]);
    setTimeout(performSearchOperation, 0);
  }, [performSearchOperation]);

  const clearSearch = React.useCallback(() => {
    setQueryState('');
    setAllResults([]);
    setDisplayedResults([]);
    setError(undefined);
    setCurrentPage(0);
  }, []);

  const loadMore = React.useCallback(() => {
    const pageSize = mergedConfig.maxResults || 50;
    const nextPage = currentPage + 1;
    const startIndex = nextPage * pageSize;
    const endIndex = startIndex + pageSize;

    if (startIndex < allResults.length) {
      setDisplayedResults(allResults.slice(0, endIndex));
      setCurrentPage(nextPage);
    }
  }, [currentPage, allResults, mergedConfig.maxResults]);

  React.useEffect(() => {
    return () => {
      if (debounceTimer.current) {
        clearTimeout(debounceTimer.current);
      }
    };
  }, []);

  const hasMore = (currentPage + 1) * (mergedConfig.maxResults || 50) < allResults.length;

  return {
    query,
    results: displayedResults,
    isSearching,
    error,
    totalResults: allResults.length,
    hasMore,
    filters,
    setQuery,
    addFilter,
    removeFilter,
    clearFilters,
    clearSearch,
    loadMore,
  };
}

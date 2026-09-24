import React from 'react';
import { Platform } from 'react-native';
import { useAppStore } from '@store/appStore';

export interface InfiniteScrollItem {
  id: string;
  title: string;
  description: string;
}

export interface InfiniteScrollState {
  items: InfiniteScrollItem[];
  /** Opaque cursor returned by the last page; null means no more pages. */
  cursor: string | null;
  hasMore: boolean;
  isLoading: boolean;
  error?: string;
  platformThreshold: number;
}

export interface UseInfiniteScrollOptions {
  pageSize?: number;
}

export interface UseInfiniteScrollResult extends InfiniteScrollState {
  loadMore: () => Promise<void>;
  refresh: () => Promise<void>;
}

// ---------------------------------------------------------------------------
// Internal helpers — simulate cursor-based backend responses.
// In production these are replaced by real API calls that return
// { items, nextCursor } where nextCursor is null on the last page.
// ---------------------------------------------------------------------------

interface CursorPage {
  items: InfiniteScrollItem[];
  nextCursor: string | null;
}

function fetchPage(cursor: string | null, pageSize: number): CursorPage {
  // Decode the offset encoded in the cursor ("cursor:<offset>") or start at 0.
  const offset = cursor ? parseInt(cursor.replace('cursor:', ''), 10) : 0;
  const maxItems = pageSize * 4; // 4 pages of data in the mock

  const items: InfiniteScrollItem[] = Array.from(
    { length: Math.min(pageSize, Math.max(0, maxItems - offset)) },
    (_, i) => {
      const n = offset + i + 1;
      return {
        id: `insight-${n}`,
        title: `Insight ${n}`,
        description: `Fetched with cursor offset ${offset}`,
      };
    },
  );

  const nextOffset = offset + items.length;
  const nextCursor = nextOffset < maxItems ? `cursor:${nextOffset}` : null;

  return { items, nextCursor };
}

// ---------------------------------------------------------------------------

export function useInfiniteScroll(
  options: UseInfiniteScrollOptions = {},
): UseInfiniteScrollResult {
  const isOnline = useAppStore(state => state.isOnline);
  const pageSize =
    options.pageSize ??
    (Platform.select({ ios: 12, android: 10, default: 10 }) ?? 10);
  const platformThreshold =
    Platform.select({ ios: 0.35, android: 0.45, default: 0.4 }) ?? 0.4;

  const initialPage = React.useMemo(() => fetchPage(null, pageSize), [pageSize]);

  const [state, setState] = React.useState<InfiniteScrollState>({
    items: initialPage.items,
    cursor: initialPage.nextCursor,
    hasMore: initialPage.nextCursor !== null,
    isLoading: false,
    platformThreshold,
  });

  const loadMore = React.useCallback(async () => {
    if (state.isLoading || !state.hasMore) {
      return;
    }

    if (!isOnline) {
      setState(s => ({
        ...s,
        error: 'Connect to the internet to load more results',
      }));
      return;
    }

    setState(s => ({ ...s, isLoading: true, error: undefined }));

    try {
      const { items: nextItems, nextCursor } = fetchPage(state.cursor, pageSize);

      setState(s => ({
        ...s,
        items: [...s.items, ...nextItems],
        cursor: nextCursor,
        hasMore: nextCursor !== null,
        isLoading: false,
      }));
    } catch {
      setState(s => ({
        ...s,
        isLoading: false,
        error: 'Unable to load more results',
      }));
    }
  }, [isOnline, pageSize, state.hasMore, state.isLoading, state.cursor]);

  const refresh = React.useCallback(async () => {
    setState(s => ({ ...s, isLoading: true, error: undefined }));

    try {
      const { items, nextCursor } = fetchPage(null, pageSize);
      setState({
        items,
        cursor: nextCursor,
        hasMore: nextCursor !== null,
        isLoading: false,
        platformThreshold,
      });
    } catch {
      setState(s => ({
        ...s,
        isLoading: false,
        error: 'Unable to refresh results',
      }));
    }
  }, [pageSize, platformThreshold]);

  return {
    ...state,
    loadMore,
    refresh,
  };
}

import { useRouter, useSearchParams, usePathname } from 'next/navigation';
import { useCallback, useMemo } from 'react';

export function usePagination(totalItems: number, defaultPageSize = 10) {
    const router = useRouter();
    const pathname = usePathname();
    const searchParams = useSearchParams();

    const currentPage = useMemo(() => {
        const page = searchParams.get('page');
        return page ? parseInt(page) : 1;
    }, [searchParams]);

    const pageSize = useMemo(() => {
        const size = searchParams.get('pageSize');
        return size ? parseInt(size) : defaultPageSize;
    }, [searchParams, defaultPageSize]);

    const setPagination = useCallback((page: number, size: number) => {
        const params = new URLSearchParams(searchParams.toString());
        params.set('page', page.toString());
        params.set('pageSize', size.toString());
        router.push(`${pathname}?${params.toString()}`);
    }, [router, pathname, searchParams]);

    const onPageChange = useCallback((page: number) => {
        setPagination(page, pageSize);
    }, [pageSize, setPagination]);

    const onPageSizeChange = useCallback((size: number) => {
        setPagination(1, size); // Reset to page 1 when size changes
    }, [setPagination]);

    const startIndex = (currentPage - 1) * pageSize;
    const endIndex = Math.min(startIndex + pageSize, totalItems);

    return {
        currentPage,
        pageSize,
        onPageChange,
        onPageSizeChange,
        startIndex,
        endIndex,
    };
}

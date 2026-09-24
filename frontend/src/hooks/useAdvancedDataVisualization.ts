import { useState, useCallback, useEffect } from 'react';

interface DataPoint {
  id?: string | number;
  name: string;
  value: number;
  [key: string]: string | number | undefined;
}

interface UseAdvancedDataVisualizationOptions {
  initialData?: DataPoint[];
  autoRefresh?: boolean;
  refreshInterval?: number;
}

export const useAdvancedDataVisualization = (options: UseAdvancedDataVisualizationOptions = {}) => {
  const { initialData = [], autoRefresh = false, refreshInterval = 5000 } = options;

  const [data, setData] = useState<DataPoint[]>(initialData);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [chartType, setChartType] = useState<'line' | 'bar' | 'pie'>('line');

  const updateData = useCallback((newData: DataPoint[]) => {
    try {
      setData(newData);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to update data');
    }
  }, []);

  const addDataPoint = useCallback((point: DataPoint) => {
    setData((prevData) => {
      if (point.id !== undefined && prevData.some((p) => p.id === point.id)) {
        return prevData;
      }
      return [...prevData, point];
    });
  }, []);

  const addDataPoints = useCallback((points: DataPoint[]) => {
    setData((prevData) => {
      const seenIds = new Set(prevData.map((p) => p.id).filter((id) => id !== undefined));
      const fresh = points.filter((p) => p.id === undefined || !seenIds.has(p.id));
      return fresh.length > 0 ? [...prevData, ...fresh] : prevData;
    });
  }, []);

  const removeDataPoint = useCallback((index: number) => {
    setData((prevData) => prevData.filter((_, i) => i !== index));
  }, []);

  const clearData = useCallback(() => {
    setData([]);
  }, []);

  const fetchData = useCallback(async (url: string) => {
    setIsLoading(true);
    try {
      const response = await fetch(url);
      if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
      const result = await response.json();
      setData(result);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch data');
    } finally {
      setIsLoading(false);
    }
  }, []);

  const transformData = useCallback((transformer: (data: DataPoint[]) => DataPoint[]) => {
    try {
      const transformed = transformer(data);
      setData(transformed);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to transform data');
    }
  }, [data]);

  const filterData = useCallback((predicate: (point: DataPoint) => boolean) => {
    const filtered = data.filter(predicate);
    setData(filtered);
  }, [data]);

  const sortData = useCallback((key: string, ascending = true) => {
    const sorted = [...data].sort((a, b) => {
      const aVal = a[key];
      const bVal = b[key];
      if (typeof aVal === 'number' && typeof bVal === 'number') {
        return ascending ? aVal - bVal : bVal - aVal;
      }
      return 0;
    });
    setData(sorted);
  }, [data]);

  useEffect(() => {
    if (!autoRefresh) return;

    const interval = setInterval(() => {
      // Auto-refresh logic can be implemented here
    }, refreshInterval);

    return () => clearInterval(interval);
  }, [autoRefresh, refreshInterval]);

  return {
    data,
    isLoading,
    error,
    chartType,
    setChartType,
    updateData,
    addDataPoint,
    addDataPoints,
    removeDataPoint,
    clearData,
    fetchData,
    transformData,
    filterData,
    sortData,
  };
};

export default useAdvancedDataVisualization;

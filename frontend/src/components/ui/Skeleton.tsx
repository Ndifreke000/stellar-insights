import React from 'react';

interface SkeletonProps {
  className?: string;
  variant?: 'text' | 'circle' | 'rect' | 'card';
}

export const Skeleton: React.FC<SkeletonProps> = ({ 
  className = '', 
  variant = 'rect' 
}) => {
  const baseStyles = 'animate-pulse bg-gray-200 dark:bg-gray-700';
  
  const variantStyles = {
    text: 'h-4 rounded',
    circle: 'rounded-full',
    rect: 'rounded',
    card: 'rounded-lg',
  };

  return (
    <div 
      className={`${baseStyles} ${variantStyles[variant]} ${className}`}
      aria-hidden="true"
    />
  );
};

export const SkeletonText: React.FC<{ lines?: number; className?: string }> = ({ 
  lines = 1, 
  className = '' 
}) => (
  <div className={`space-y-2 ${className}`}>
    {Array.from({ length: lines }).map((_, i) => (
      <Skeleton 
        key={i} 
        variant="text" 
        className={i === lines - 1 ? 'w-4/5' : 'w-full'} 
      />
    ))}
  </div>
);

export const SkeletonCard: React.FC<{ className?: string }> = ({ className = '' }) => (
  <div className={`bg-white dark:bg-slate-800 rounded-lg border border-gray-200 dark:border-slate-700 p-6 ${className}`}>
    <div className="flex items-start justify-between mb-4">
      <Skeleton variant="circle" className="w-10 h-10" />
    </div>
    <SkeletonText lines={1} className="mb-2" />
    <Skeleton className="h-8 w-24 mb-2" />
    <SkeletonText lines={1} className="w-32" />
  </div>
);

export const SkeletonCorridorCard: React.FC<{ className?: string }> = ({ className = '' }) => (
  <div className={`bg-white dark:bg-slate-800 rounded-lg shadow-sm border border-gray-200 dark:border-slate-700 p-6 ${className}`}>
    <div className="flex justify-between items-start mb-4">
      <div className="flex-1">
        <Skeleton className="h-6 w-32 mb-2" />
        <SkeletonText lines={1} className="w-48" />
      </div>
      <Skeleton variant="circle" className="w-8 h-8" />
    </div>
    <div className="space-y-3 mb-4">
      <div className="flex justify-between">
        <SkeletonText lines={1} className="w-24" />
        <Skeleton className="h-4 w-16" />
      </div>
      <div className="flex justify-between">
        <SkeletonText lines={1} className="w-24" />
        <Skeleton className="h-4 w-16" />
      </div>
      <div className="flex justify-between">
        <SkeletonText lines={1} className="w-24" />
        <Skeleton className="h-4 w-16" />
      </div>
    </div>
    <Skeleton className="h-10 w-full rounded-lg" />
  </div>
);

export const SkeletonMetricsCard: React.FC<{ className?: string }> = ({ className = '' }) => (
  <div className={`bg-white rounded shadow p-4 ${className}`}>
    <SkeletonText lines={1} className="w-40 mb-4" />
    <Skeleton className="h-32 w-full mb-4" />
    <div className="grid grid-cols-2 gap-4">
      <div>
        <SkeletonText lines={1} className="w-20 mb-2" />
        <Skeleton className="h-6 w-24" />
      </div>
      <div>
        <SkeletonText lines={1} className="w-20 mb-2" />
        <Skeleton className="h-6 w-24" />
      </div>
    </div>
  </div>
);

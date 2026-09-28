import { SkeletonCard, SkeletonChart } from "@/components/ui/Skeleton";

/**
 * Next.js App Router loading UI for the Analytics route segment.
 * Shown instantly during navigation before the page component mounts (#2334).
 */
export default function AnalyticsLoading() {
  return (
    <div className="space-y-8 animate-in fade-in duration-300">
      {/* Header skeleton */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div className="space-y-2">
          <div className="h-3 w-36 bg-accent/20 rounded animate-pulse" />
          <div className="h-10 w-56 bg-white/10 rounded animate-pulse" />
        </div>
        <div className="flex items-center gap-3">
          <div className="h-8 w-36 bg-white/5 border border-border/20 rounded-lg animate-pulse" />
          <div className="h-8 w-24 bg-accent/20 rounded-lg animate-pulse" />
        </div>
      </div>
      {/* Metric cards skeleton */}
      <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <SkeletonCard key={i} />
        ))}
      </div>
      {/* Charts skeleton */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-6">
        <div className="lg:col-span-8 space-y-6">
          <SkeletonChart height={256} />
          <SkeletonChart height={256} />
          <SkeletonChart height={256} />
        </div>
        <div className="lg:col-span-4 space-y-6">
          <SkeletonChart height={256} />
          <SkeletonChart height={256} />
        </div>
      </div>
    </div>
  );
}

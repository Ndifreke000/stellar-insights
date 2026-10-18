import { SkeletonCard } from "@/components/ui/Skeleton";

/**
 * Next.js App Router loading UI for the Governance route segment.
 * Shown instantly during navigation before the page component mounts (#2334).
 */
export default function GovernanceLoading() {
  return (
    <div className="space-y-8 animate-in fade-in duration-300">
      {/* Header skeleton */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div className="space-y-2">
          <div className="h-3 w-36 bg-accent/20 rounded animate-pulse" />
          <div className="h-10 w-44 bg-white/10 rounded animate-pulse" />
        </div>
      </div>
      {/* Metric cards skeleton */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <SkeletonCard key={i} />
        ))}
      </div>
      {/* Tab row skeleton */}
      <div className="flex items-center gap-2">
        {[1, 2, 3, 4, 5].map((i) => (
          <div
            key={i}
            className="h-8 w-16 bg-white/5 border border-border/20 rounded-xl animate-pulse"
          />
        ))}
      </div>
      {/* Proposal cards skeleton */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {[1, 2, 3, 4].map((i) => (
          <div
            key={i}
            className="glass-card rounded-2xl p-6 border border-border/50 space-y-4 animate-pulse"
          >
            <div className="flex items-start justify-between">
              <div className="space-y-2 flex-1">
                <div className="h-5 w-3/4 bg-white/10 rounded" />
                <div className="h-3 w-1/2 bg-white/5 rounded" />
              </div>
              <div className="h-6 w-16 bg-accent/20 rounded-full" />
            </div>
            <div className="h-3 w-full bg-white/5 rounded" />
            <div className="h-3 w-4/5 bg-white/5 rounded" />
            <div className="h-2 w-full bg-white/5 rounded-full" />
            <div className="flex gap-2">
              <div className="h-8 flex-1 bg-white/5 rounded-lg" />
              <div className="h-8 flex-1 bg-white/5 rounded-lg" />
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

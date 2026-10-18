import { SkeletonCorridorCard } from "@/components/ui/Skeleton";

/**
 * Next.js App Router loading UI for the Corridors route segment.
 * Shown instantly during navigation before the page component mounts (#2334).
 */
export default function CorridorsLoading() {
  return (
    <div className="space-y-8 animate-in fade-in duration-300">
      {/* Header skeleton */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div className="space-y-2">
          <div className="h-3 w-40 bg-accent/20 rounded animate-pulse" />
          <div className="h-10 w-56 bg-white/10 rounded animate-pulse" />
        </div>
        <div className="flex items-center gap-3">
          <div className="h-8 w-28 bg-white/5 border border-border/20 rounded-xl animate-pulse" />
          <div className="h-8 w-28 bg-white/5 border border-border/20 rounded-xl animate-pulse" />
        </div>
      </div>
      {/* Filters skeleton */}
      <div className="grid grid-cols-1 lg:grid-cols-12 gap-4">
        <div className="lg:col-span-8 h-12 bg-white/5 border border-border/20 rounded-xl animate-pulse" />
        <div className="lg:col-span-4 flex gap-2">
          <div className="flex-1 h-12 bg-white/5 border border-border/20 rounded-xl animate-pulse" />
          <div className="flex-1 h-12 bg-white/5 border border-border/20 rounded-xl animate-pulse" />
        </div>
      </div>
      {/* Cards skeleton */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {[1, 2, 3, 4, 5, 6].map((i) => (
          <SkeletonCorridorCard key={i} />
        ))}
      </div>
    </div>
  );
}

import { Skeleton, SkeletonTable } from "@/components/ui/Skeleton";

/**
 * Next.js App Router loading UI for the Anchors route segment.
 * Shown instantly during navigation before the page component mounts (#2334).
 */
export default function AnchorsLoading() {
  return (
    <div className="space-y-6 animate-in fade-in duration-300">
      <div className="flex items-center gap-4 mb-8">
        <Skeleton variant="circle" className="w-12 h-12" />
        <div className="space-y-2">
          <Skeleton className="h-4 w-32" />
          <Skeleton className="h-6 w-48" />
        </div>
      </div>
      <SkeletonTable rows={10} />
    </div>
  );
}

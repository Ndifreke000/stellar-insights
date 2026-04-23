import { Suspense } from "react";
import { Home as AnchorIcon } from "lucide-react";
import { MainLayout } from "@/components/layout";
import AnchorsPageContent from "./components/AnchorsPageContent";
import { ErrorBoundary } from "@/components/ErrorBoundary";
import { Skeleton, SkeletonTable } from "@/components/ui/Skeleton";

const AnchorsPage = () => {
  return (
    <ErrorBoundary>
      <Suspense
        fallback={
          <MainLayout>
            <div className="space-y-6">
              <div className="flex items-center gap-4 mb-8">
                <Skeleton variant="circle" className="w-12 h-12" />
                <div className="space-y-2">
                  <Skeleton className="h-4 w-32" />
                  <Skeleton className="h-6 w-48" />
                </div>
              </div>
              <SkeletonTable rows={10} />
            </div>
          </MainLayout>
        }
      >
        <AnchorsPageContent />
      </Suspense>
    </ErrorBoundary>
  );
};

export default AnchorsPage;
import { Suspense } from "react";
import { Home as AnchorIcon } from "lucide-react";
import { MainLayout } from "@/components/layout";
import AnchorsPageContent from "./components/AnchorsPageContent";
import { ErrorBoundary } from "@/components/ErrorBoundary";

const AnchorsPage = () => {
  return (
    <ErrorBoundary>
      <Suspense
        fallback={
          <MainLayout>
            <div className="flex items-center justify-center min-h-screen">
              <div className="text-center">
                <AnchorIcon className="w-12 h-12 text-gray-400 mx-auto mb-4 animate-pulse" />
                <p className="text-gray-600 dark:text-gray-400">
                  Loading anchors...
                </p>
              </div>
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
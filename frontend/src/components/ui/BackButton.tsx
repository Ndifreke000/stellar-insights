"use client";

import { ArrowLeft } from "lucide-react";
import { useRouter } from "@/i18n/navigation";

interface BackButtonProps {
  fallbackHref: string;
  label?: string;
  className?: string;
}

export function BackButton({
  fallbackHref,
  label = "Back",
  className = "flex items-center gap-2 text-blue-600 dark:text-link-primary hover:text-blue-700 dark:hover:text-blue-300 transition-colors font-medium group",
}: BackButtonProps) {
  const router = useRouter();

  const handleBack = () => {
    if (typeof window !== "undefined" && window.history.length > 1) {
      router.back();
    } else {
      router.push(fallbackHref);
    }
  };

  return (
    <button onClick={handleBack} className={className}>
      <ArrowLeft className="w-5 h-5 transition-transform group-hover:-translate-x-1" />
      {label}
    </button>
  );
}

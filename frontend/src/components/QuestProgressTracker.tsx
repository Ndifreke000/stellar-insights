"use client";

import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { checkPathCompletion } from "@/lib/quests";

/**
 * Tracks navigation and marks quests complete when user visits matching paths
 */
export function QuestProgressTracker() {
  const pathname = usePathname();

  useEffect(() => {
    if (pathname) checkPathCompletion(pathname);
  }, [pathname]);

  return null;
}

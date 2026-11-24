"use client";

import { usePathname } from "next/navigation";
import { useEffect } from "react";
import { reportActivity } from "@/lib/quests";

/**
 * Tracks navigation and reports it to the backend, which decides quest completion
 */
export function QuestProgressTracker() {
  const pathname = usePathname();

  useEffect(() => {
    if (pathname) void reportActivity(pathname);
  }, [pathname]);

  return null;
}

"use client";

import React from "react";
import { Bookmark, BookmarkCheck } from "lucide-react";
import { useBookmarks, BookmarkType } from "@/hooks/useBookmarks";

interface BookmarkButtonProps {
  id: string;
  type: BookmarkType;
  label: string;
  href: string;
  /** Extra CSS classes for the button wrapper */
  className?: string;
  /** Show label text alongside icon */
  showLabel?: boolean;
}

export function BookmarkButton({
  id,
  type,
  label,
  href,
  className = "",
  showLabel = false,
}: BookmarkButtonProps) {
  const { isBookmarked, toggleBookmark } = useBookmarks();
  const active = isBookmarked(id, type);

  const handleClick = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    toggleBookmark({ id, type, label, href });
  };

  return (
    <button
      onClick={handleClick}
      aria-pressed={active}
      aria-label={active ? `Remove ${label} from bookmarks` : `Bookmark ${label}`}
      title={active ? "Remove bookmark" : "Add bookmark"}
      className={`
        inline-flex items-center gap-1.5 px-2 py-1 rounded-lg
        text-[10px] font-mono uppercase tracking-wider
        transition-all duration-200
        ${active
          ? "text-accent bg-accent/10 border border-accent/30 hover:bg-accent/20"
          : "text-muted-foreground bg-white/5 border border-white/10 hover:border-accent/40 hover:text-accent"
        }
        ${className}
      `}
    >
      {active ? (
        <BookmarkCheck className="w-3 h-3 shrink-0" aria-hidden="true" />
      ) : (
        <Bookmark className="w-3 h-3 shrink-0" aria-hidden="true" />
      )}
      {showLabel && (
        <span>{active ? "Bookmarked" : "Bookmark"}</span>
      )}
    </button>
  );
}

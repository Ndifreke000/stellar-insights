"use client";

import React from "react";
import { Bookmark, Compass, Anchor, ArrowRight } from "lucide-react";
import { Link } from "@/i18n/navigation";
import { useBookmarks } from "@/hooks/useBookmarks";

/**
 * Dashboard widget showing the user's bookmarked corridors and anchors.
 * Renders nothing if there are no bookmarks.
 */
export function BookmarksDashboardWidget() {
  const { bookmarks, corridorBookmarks, anchorBookmarks } = useBookmarks();

  if (bookmarks.length === 0) return null;

  return (
    <div className="glass-card rounded-2xl p-6 space-y-4">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Bookmark className="w-4 h-4 text-accent" aria-hidden="true" />
          <h3 className="text-[10px] font-mono uppercase tracking-[0.2em] text-accent">
            Bookmarks
          </h3>
        </div>
        <span className="text-[10px] font-mono text-muted-foreground">
          {bookmarks.length} saved
        </span>
      </div>

      <div className="space-y-3">
        {corridorBookmarks.length > 0 && (
          <div>
            <p className="text-[9px] font-mono uppercase tracking-widest text-muted-foreground/50 mb-2">
              Corridors
            </p>
            <ul role="list" className="space-y-1">
              {corridorBookmarks.slice(0, 3).map((b) => (
                <li key={`${b.type}-${b.id}`}>
                  <Link
                    href={b.href}
                    className="flex items-center justify-between group px-3 py-2 rounded-lg hover:bg-white/5 transition-colors"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Compass
                        className="w-3.5 h-3.5 shrink-0 text-muted-foreground"
                        aria-hidden="true"
                      />
                      <span className="text-sm font-medium text-foreground truncate">
                        {b.label}
                      </span>
                    </div>
                    <ArrowRight
                      className="w-3.5 h-3.5 shrink-0 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
                      aria-hidden="true"
                    />
                  </Link>
                </li>
              ))}
              {corridorBookmarks.length > 3 && (
                <li className="px-3 py-1 text-[10px] font-mono text-muted-foreground">
                  +{corridorBookmarks.length - 3} more
                </li>
              )}
            </ul>
          </div>
        )}

        {anchorBookmarks.length > 0 && (
          <div>
            <p className="text-[9px] font-mono uppercase tracking-widest text-muted-foreground/50 mb-2">
              Anchors
            </p>
            <ul role="list" className="space-y-1">
              {anchorBookmarks.slice(0, 3).map((b) => (
                <li key={`${b.type}-${b.id}`}>
                  <Link
                    href={b.href}
                    className="flex items-center justify-between group px-3 py-2 rounded-lg hover:bg-white/5 transition-colors"
                  >
                    <div className="flex items-center gap-2 min-w-0">
                      <Anchor
                        className="w-3.5 h-3.5 shrink-0 text-muted-foreground"
                        aria-hidden="true"
                      />
                      <span className="text-sm font-medium text-foreground truncate">
                        {b.label}
                      </span>
                    </div>
                    <ArrowRight
                      className="w-3.5 h-3.5 shrink-0 text-muted-foreground opacity-0 group-hover:opacity-100 transition-opacity"
                      aria-hidden="true"
                    />
                  </Link>
                </li>
              ))}
              {anchorBookmarks.length > 3 && (
                <li className="px-3 py-1 text-[10px] font-mono text-muted-foreground">
                  +{anchorBookmarks.length - 3} more
                </li>
              )}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}

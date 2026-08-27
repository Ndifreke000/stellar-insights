"use client";

import React, { useState } from "react";
import { Bookmark, Compass, Anchor, Trash2, ChevronDown } from "lucide-react";
import { Link } from "@/i18n/navigation";
import { useBookmarks } from "@/hooks/useBookmarks";

export function BookmarksSidebarSection({ collapsed }: { collapsed: boolean }) {
  const { bookmarks, removeBookmark, corridorBookmarks, anchorBookmarks } =
    useBookmarks();
  const [open, setOpen] = useState(true);

  if (bookmarks.length === 0) return null;

  if (collapsed) {
    // In icon-only mode just show a small indicator if there are bookmarks
    return (
      <div className="px-4 py-2">
        <div
          className="flex items-center justify-center"
          title={`${bookmarks.length} bookmark${bookmarks.length !== 1 ? "s" : ""}`}
        >
          <Bookmark className="w-5 h-5 text-accent" aria-hidden="true" />
        </div>
      </div>
    );
  }

  return (
    <div className="mt-4">
      <button
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        className="w-full flex items-center justify-between px-4 py-1.5 text-[10px] font-bold uppercase tracking-widest text-muted-foreground/70 hover:text-foreground transition-colors"
      >
        <span className="flex items-center gap-1.5">
          <Bookmark className="w-3 h-3" aria-hidden="true" />
          Bookmarks
        </span>
        <ChevronDown
          aria-hidden="true"
          className={`w-3.5 h-3.5 shrink-0 transition-transform duration-200 ${open ? "" : "-rotate-90"}`}
        />
      </button>

      {open && (
        <ul role="list" className="space-y-1 m-0 p-0 list-none mt-1">
          {corridorBookmarks.length > 0 && (
            <>
              <li className="px-4 pt-1 pb-0.5 text-[9px] font-mono uppercase tracking-widest text-muted-foreground/50">
                Corridors
              </li>
              {corridorBookmarks.map((b) => (
                <li key={`${b.type}-${b.id}`} className="group">
                  <div className="flex items-center gap-1 px-4 py-1.5 rounded-xl hover:bg-white/5 transition-colors">
                    <Compass
                      className="w-3.5 h-3.5 shrink-0 text-muted-foreground"
                      aria-hidden="true"
                    />
                    <Link
                      href={b.href}
                      className="flex-1 text-[11px] font-mono text-muted-foreground hover:text-foreground truncate transition-colors"
                    >
                      {b.label}
                    </Link>
                    <button
                      onClick={() => removeBookmark(b.id, b.type)}
                      aria-label={`Remove ${b.label} from bookmarks`}
                      className="opacity-0 group-hover:opacity-100 transition-opacity p-0.5 rounded hover:text-red-400 text-muted-foreground"
                    >
                      <Trash2 className="w-3 h-3" aria-hidden="true" />
                    </button>
                  </div>
                </li>
              ))}
            </>
          )}

          {anchorBookmarks.length > 0 && (
            <>
              <li className="px-4 pt-1 pb-0.5 text-[9px] font-mono uppercase tracking-widest text-muted-foreground/50">
                Anchors
              </li>
              {anchorBookmarks.map((b) => (
                <li key={`${b.type}-${b.id}`} className="group">
                  <div className="flex items-center gap-1 px-4 py-1.5 rounded-xl hover:bg-white/5 transition-colors">
                    <Anchor
                      className="w-3.5 h-3.5 shrink-0 text-muted-foreground"
                      aria-hidden="true"
                    />
                    <Link
                      href={b.href}
                      className="flex-1 text-[11px] font-mono text-muted-foreground hover:text-foreground truncate transition-colors"
                    >
                      {b.label}
                    </Link>
                    <button
                      onClick={() => removeBookmark(b.id, b.type)}
                      aria-label={`Remove ${b.label} from bookmarks`}
                      className="opacity-0 group-hover:opacity-100 transition-opacity p-0.5 rounded hover:text-red-400 text-muted-foreground"
                    >
                      <Trash2 className="w-3 h-3" aria-hidden="true" />
                    </button>
                  </div>
                </li>
              ))}
            </>
          )}
        </ul>
      )}
    </div>
  );
}

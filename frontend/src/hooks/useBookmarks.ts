"use client";

import { useCallback, useMemo } from "react";
import { useLocalStorage } from "./useLocalStorage";

export type BookmarkType = "corridor" | "anchor";

export interface Bookmark {
  id: string;
  type: BookmarkType;
  label: string;
  href: string;
  addedAt: string; // ISO timestamp
}

const STORAGE_KEY = "stellar-bookmarks";

export function useBookmarks() {
  const [bookmarks, setBookmarks] = useLocalStorage<Bookmark[]>(STORAGE_KEY, []);

  const addBookmark = useCallback(
    (bookmark: Omit<Bookmark, "addedAt">) => {
      setBookmarks((prev) => {
        if (prev.some((b) => b.id === bookmark.id && b.type === bookmark.type)) {
          return prev; // already bookmarked
        }
        return [...prev, { ...bookmark, addedAt: new Date().toISOString() }];
      });
    },
    [setBookmarks]
  );

  const removeBookmark = useCallback(
    (id: string, type: BookmarkType) => {
      setBookmarks((prev) =>
        prev.filter((b) => !(b.id === id && b.type === type))
      );
    },
    [setBookmarks]
  );

  const isBookmarked = useCallback(
    (id: string, type: BookmarkType) =>
      bookmarks.some((b) => b.id === id && b.type === type),
    [bookmarks]
  );

  const toggleBookmark = useCallback(
    (bookmark: Omit<Bookmark, "addedAt">) => {
      if (isBookmarked(bookmark.id, bookmark.type)) {
        removeBookmark(bookmark.id, bookmark.type);
      } else {
        addBookmark(bookmark);
      }
    },
    [isBookmarked, addBookmark, removeBookmark]
  );

  const corridorBookmarks = useMemo(
    () => bookmarks.filter((b) => b.type === "corridor"),
    [bookmarks]
  );

  const anchorBookmarks = useMemo(
    () => bookmarks.filter((b) => b.type === "anchor"),
    [bookmarks]
  );

  return {
    bookmarks,
    corridorBookmarks,
    anchorBookmarks,
    addBookmark,
    removeBookmark,
    isBookmarked,
    toggleBookmark,
  };
}

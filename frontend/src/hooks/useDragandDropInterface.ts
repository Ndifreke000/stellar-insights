import { useState, useCallback } from 'react';

interface DraggableItem {
  id: string;
  content: string;
  order: number;
}

interface UseDragandDropInterfaceOptions {
  initialItems?: DraggableItem[];
  onItemsChange?: (items: DraggableItem[]) => void;
}

export const useDragandDropInterface = (options: UseDragandDropInterfaceOptions = {}) => {
  const { initialItems = [], onItemsChange } = options;

  const [items, setItems] = useState<DraggableItem[]>(initialItems);
  const [draggedItem, setDraggedItem] = useState<DraggableItem | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);

  const reorderItems = useCallback((newItems: DraggableItem[]) => {
    const reordered = newItems.map((item, index) => ({
      ...item,
      order: index,
    }));
    setItems(reordered);
    onItemsChange?.(reordered);
  }, [onItemsChange]);

  const moveItem = useCallback((fromIndex: number, toIndex: number) => {
    const newItems = [...items];
    const [removed] = newItems.splice(fromIndex, 1);
    newItems.splice(toIndex, 0, removed);
    reorderItems(newItems);
  }, [items, reorderItems]);

  const addItem = useCallback((content: string) => {
    const newItem: DraggableItem = {
      id: `item-${Date.now()}`,
      content,
      order: items.length,
    };
    const newItems = [...items, newItem];
    reorderItems(newItems);
    return newItem;
  }, [items, reorderItems]);

  const deleteItem = useCallback((id: string) => {
    const newItems = items.filter((item) => item.id !== id);
    reorderItems(newItems);
  }, [items, reorderItems]);

  const updateItem = useCallback((id: string, content: string) => {
    const newItems = items.map((item) =>
      item.id === id ? { ...item, content } : item
    );
    setItems(newItems);
    onItemsChange?.(newItems);
  }, [items, onItemsChange]);

  const clearItems = useCallback(() => {
    setItems([]);
    onItemsChange?.([]);
  }, [onItemsChange]);

  const duplicateItem = useCallback((id: string) => {
    const itemToDuplicate = items.find((item) => item.id === id);
    if (!itemToDuplicate) return;

    const newItem: DraggableItem = {
      id: `item-${Date.now()}`,
      content: `${itemToDuplicate.content} (copy)`,
      order: items.length,
    };
    const newItems = [...items, newItem];
    reorderItems(newItems);
    return newItem;
  }, [items, reorderItems]);

  const sortItems = useCallback((compareFn?: (a: DraggableItem, b: DraggableItem) => number) => {
    const sorted = [...items].sort(compareFn || ((a, b) => a.content.localeCompare(b.content)));
    reorderItems(sorted);
  }, [items, reorderItems]);

  const filterItems = useCallback((predicate: (item: DraggableItem) => boolean) => {
    const filtered = items.filter(predicate);
    reorderItems(filtered);
  }, [items, reorderItems]);

  return {
    items,
    draggedItem,
    setDraggedItem,
    dragOverIndex,
    setDragOverIndex,
    moveItem,
    addItem,
    deleteItem,
    updateItem,
    clearItems,
    duplicateItem,
    sortItems,
    filterItems,
  };
};

export default useDragandDropInterface;

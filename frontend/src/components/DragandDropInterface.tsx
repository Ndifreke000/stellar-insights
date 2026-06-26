'use client';

import React, { useState, useCallback, useRef } from 'react';
import { GripVertical, Trash2, Plus } from 'lucide-react';

interface DraggableItem {
  id: string;
  content: string;
  order: number;
}

interface DragandDropInterfaceProps {
  items?: DraggableItem[];
  onItemsChange?: (items: DraggableItem[]) => void;
  onItemDelete?: (id: string) => void;
  onItemAdd?: (item: DraggableItem) => void;
  title?: string;
  allowAdd?: boolean;
  allowDelete?: boolean;
}

export const DragandDropInterface: React.FC<DragandDropInterfaceProps> = ({
  items: initialItems = [],
  onItemsChange,
  onItemDelete,
  onItemAdd,
  title = 'Drag and Drop Interface',
  allowAdd = true,
  allowDelete = true,
}) => {
  const [items, setItems] = useState<DraggableItem[]>(initialItems);
  const [draggedItem, setDraggedItem] = useState<DraggableItem | null>(null);
  const [dragOverIndex, setDragOverIndex] = useState<number | null>(null);
  const [newItemContent, setNewItemContent] = useState('');
  const [showAddForm, setShowAddForm] = useState(false);
  const dragImageRef = useRef<HTMLDivElement>(null);

  const handleDragStart = (e: React.DragEvent, item: DraggableItem, index: number) => {
    setDraggedItem(item);
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', item.content);
    
    if (dragImageRef.current) {
      e.dataTransfer.setDragImage(dragImageRef.current, 0, 0);
    }
  };

  const handleDragOver = (e: React.DragEvent, index: number) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = 'move';
    setDragOverIndex(index);
  };

  const handleDragLeave = () => {
    setDragOverIndex(null);
  };

  const handleDrop = (e: React.DragEvent, targetIndex: number) => {
    e.preventDefault();
    setDragOverIndex(null);

    if (!draggedItem) return;

    const draggedIndex = items.findIndex((item) => item.id === draggedItem.id);
    if (draggedIndex === -1) return;

    const newItems = [...items];
    const [removed] = newItems.splice(draggedIndex, 1);
    newItems.splice(targetIndex, 0, removed);

    const reorderedItems = newItems.map((item, index) => ({
      ...item,
      order: index,
    }));

    setItems(reorderedItems);
    onItemsChange?.(reorderedItems);
    setDraggedItem(null);
  };

  const handleDeleteItem = (id: string) => {
    const newItems = items.filter((item) => item.id !== id);
    const reorderedItems = newItems.map((item, index) => ({
      ...item,
      order: index,
    }));
    setItems(reorderedItems);
    onItemDelete?.(id);
    onItemsChange?.(reorderedItems);
  };

  const handleAddItem = () => {
    if (!newItemContent.trim()) return;

    const newItem: DraggableItem = {
      id: `item-${Date.now()}`,
      content: newItemContent,
      order: items.length,
    };

    const newItems = [...items, newItem];
    setItems(newItems);
    onItemAdd?.(newItem);
    onItemsChange?.(newItems);
    setNewItemContent('');
    setShowAddForm(false);
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleAddItem();
    }
  };

  return (
    <div className="w-full bg-white rounded-lg shadow-lg p-6" role="region" aria-label={title}>
      <div className="mb-6">
        <h2 className="text-2xl font-bold text-gray-800 mb-4">{title}</h2>
        <p className="text-sm text-gray-600">
          Drag items to reorder them. {allowDelete && 'Click the trash icon to delete.'} {allowAdd && 'Click the plus icon to add new items.'}
        </p>
      </div>

      {/* Items List */}
      <div className="space-y-2 mb-6 min-h-96 bg-gray-50 rounded-lg p-4">
        {items.length === 0 ? (
          <div className="flex items-center justify-center h-64 text-gray-400">
            <p>No items yet. {allowAdd && 'Add one to get started!'}</p>
          </div>
        ) : (
          items.map((item, index) => (
            <div
              key={item.id}
              draggable
              onDragStart={(e) => handleDragStart(e, item, index)}
              onDragOver={(e) => handleDragOver(e, index)}
              onDragLeave={handleDragLeave}
              onDrop={(e) => handleDrop(e, index)}
              className={`flex items-center gap-3 p-4 bg-white rounded-lg border-2 transition-all cursor-move group ${
                dragOverIndex === index
                  ? 'border-blue-500 bg-blue-50 scale-105'
                  : draggedItem?.id === item.id
                  ? 'border-blue-300 opacity-50'
                  : 'border-gray-200 hover:border-gray-300'
              }`}
              role="listitem"
              aria-label={`Item: ${item.content}`}
            >
              <GripVertical
                size={20}
                className="text-gray-400 flex-shrink-0"
                aria-hidden="true"
              />
              <span className="flex-1 text-gray-800 font-medium">{item.content}</span>
              <span className="text-xs text-gray-500 bg-gray-100 px-2 py-1 rounded">
                #{item.order + 1}
              </span>
              {allowDelete && (
                <button
                  onClick={() => handleDeleteItem(item.id)}
                  className="opacity-0 group-hover:opacity-100 transition-opacity p-2 hover:bg-red-50 rounded-lg text-red-600"
                  aria-label={`Delete item: ${item.content}`}
                >
                  <Trash2 size={18} />
                </button>
              )}
            </div>
          ))
        )}
      </div>

      {/* Add Item Form */}
      {allowAdd && (
        <div className="space-y-3">
          {!showAddForm ? (
            <button
              onClick={() => setShowAddForm(true)}
              className="w-full flex items-center justify-center gap-2 px-4 py-3 bg-blue-500 hover:bg-blue-600 text-white rounded-lg transition-colors font-medium"
              aria-label="Add new item"
            >
              <Plus size={20} />
              Add New Item
            </button>
          ) : (
            <div className="flex gap-2">
              <input
                type="text"
                value={newItemContent}
                onChange={(e) => setNewItemContent(e.target.value)}
                onKeyPress={handleKeyPress}
                placeholder="Enter item content..."
                className="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                autoFocus
                aria-label="New item content"
              />
              <button
                onClick={handleAddItem}
                disabled={!newItemContent.trim()}
                className="px-4 py-2 bg-green-500 hover:bg-green-600 disabled:bg-gray-300 text-white rounded-lg transition-colors font-medium"
                aria-label="Confirm add item"
              >
                Add
              </button>
              <button
                onClick={() => {
                  setShowAddForm(false);
                  setNewItemContent('');
                }}
                className="px-4 py-2 bg-gray-300 hover:bg-gray-400 text-gray-800 rounded-lg transition-colors font-medium"
                aria-label="Cancel add item"
              >
                Cancel
              </button>
            </div>
          )}
        </div>
      )}

      {/* Stats */}
      <div className="mt-6 grid grid-cols-2 md:grid-cols-3 gap-4">
        <div className="bg-blue-50 p-4 rounded-lg">
          <p className="text-sm text-gray-600">Total Items</p>
          <p className="text-2xl font-bold text-blue-600">{items.length}</p>
        </div>
        <div className="bg-green-50 p-4 rounded-lg">
          <p className="text-sm text-gray-600">Status</p>
          <p className="text-2xl font-bold text-green-600">Ready</p>
        </div>
        {draggedItem && (
          <div className="bg-yellow-50 p-4 rounded-lg">
            <p className="text-sm text-gray-600">Dragging</p>
            <p className="text-lg font-bold text-yellow-600 truncate">{draggedItem.content}</p>
          </div>
        )}
      </div>

      {/* Hidden drag image */}
      <div
        ref={dragImageRef}
        className="absolute -left-96 w-64 p-4 bg-white rounded-lg shadow-lg border-2 border-blue-500 pointer-events-none"
      >
        <div className="flex items-center gap-3">
          <GripVertical size={20} className="text-gray-400" />
          <span className="text-gray-800 font-medium">{draggedItem?.content}</span>
        </div>
      </div>
    </div>
  );
};

export default DragandDropInterface;

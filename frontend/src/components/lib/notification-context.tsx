"use client";

import React, { createContext, useContext, useState, useEffect, useCallback } from "react";

export type NotificationType = "info" | "success" | "warning" | "error";

export interface AppNotification {
  id: string;
  title: string;
  message: string;
  type: NotificationType;
  read: boolean;
  createdAt: number;
  actionLink?: string;
  actionText?: string;
}

interface NotificationContextProps {
  notifications: AppNotification[];
  unreadCount: number;
  addNotification: (notification: Omit<AppNotification, "id" | "read" | "createdAt">) => void;
  markAsRead: (id: string) => void;
  markAllAsRead: () => void;
  removeNotification: (id: string) => void;
  clearAll: () => void;
}

const NotificationContext = createContext<NotificationContextProps | undefined>(undefined);

// Generate a random ID for notifications
const generateId = () => Math.random().toString(36).substring(2, 9);

export function NotificationProvider({ children }: { children: React.ReactNode }) {
  const [notifications, setNotifications] = useState<AppNotification[]>([]);
  
  // Calculate unread count
  const unreadCount = notifications.filter(n => !n.read).length;

  const addNotification = useCallback((notification: Omit<AppNotification, "id" | "read" | "createdAt">) => {
    const newNotification: AppNotification = {
      ...notification,
      id: generateId(),
      read: false,
      createdAt: Date.now(),
    };
    
    setNotifications((prev) => [newNotification, ...prev]);
  }, []);

  const markAsRead = useCallback((id: string) => {
    setNotifications((prev) => 
      prev.map(n => n.id === id ? { ...n, read: true } : n)
    );
  }, []);

  const markAllAsRead = useCallback(() => {
    setNotifications((prev) => 
      prev.map(n => ({ ...n, read: true }))
    );
  }, []);

  const removeNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter(n => n.id !== id));
  }, []);

  const clearAll = useCallback(() => {
    setNotifications([]);
  }, []);

  // Demo: Add some initial mock notifications to visualize the design
  useEffect(() => {
    const mockNotifications: AppNotification[] = [
      {
        id: generateId(),
        title: "Network Congestion",
        message: "High traffic detected on the Stellar network. Settlement times may be slightly delayed.",
        type: "warning",
        read: false,
        createdAt: Date.now() - 1000 * 60 * 5, // 5 mins ago
      },
      {
        id: generateId(),
        title: "Liquidity Alert",
        message: "Liquidity pool XLM/USDC is experiencing high volatility.",
        type: "error",
        read: false,
        createdAt: Date.now() - 1000 * 60 * 30, // 30 mins ago
        actionLink: "/dashboard",
        actionText: "View Pool",
      },
      {
        id: generateId(),
        title: "System Update Complete",
        message: "The analytics engine has been successfully updated to v2.1.0.",
        type: "success",
        read: true,
        createdAt: Date.now() - 1000 * 60 * 60 * 2, // 2 hours ago
      }
    ];
    setNotifications(mockNotifications);
  }, []);

  const value = {
    notifications,
    unreadCount,
    addNotification,
    markAsRead,
    markAllAsRead,
    removeNotification,
    clearAll
  };

  return (
    <NotificationContext.Provider value={value}>
      {children}
    </NotificationContext.Provider>
  );
}

export function useNotifications() {
  const context = useContext(NotificationContext);
  if (context === undefined) {
    throw new Error("useNotifications must be used within a NotificationProvider");
  }
  return context;
}

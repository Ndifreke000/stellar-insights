"use client";

import React, { useState, useRef, useEffect } from "react";
import { Bell, Check, Trash2, X, AlertCircle, Info, CheckCircle2, AlertTriangle, ExternalLink } from "lucide-react";
import { useNotifications, NotificationType, AppNotification } from "../lib/notification-context";

export function NotificationCenter() {
    const {
        notifications,
        unreadCount,
        markAsRead,
        markAllAsRead,
        removeNotification,
        clearAll
    } = useNotifications();

    const [isOpen, setIsOpen] = useState(false);
    const [activeTab, setActiveTab] = useState<"all" | "unread">("all");
    const dropdownRef = useRef<HTMLDivElement>(null);

    // Close dropdown when clicking outside
    useEffect(() => {
        function handleClickOutside(event: MouseEvent) {
            if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
                setIsOpen(false);
            }
        }
        document.addEventListener("mousedown", handleClickOutside);
        return () => document.removeEventListener("mousedown", handleClickOutside);
    }, []);

    const filteredNotifications = notifications.filter(
        (n) => activeTab === "all" || !n.read
    );

    const formatDistanceToNow = (timestamp: number) => {
        const diff = Date.now() - timestamp;
        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(minutes / 60);
        const days = Math.floor(hours / 24);

        if (minutes < 1) return 'Just now';
        if (minutes < 60) return `${minutes}m ago`;
        if (hours < 24) return `${hours}h ago`;
        return `${days}d ago`;
    };

    const getTypeStyles = (type: NotificationType) => {
        switch (type) {
            case "info":
                return {
                    icon: <Info className="w-5 h-5 text-blue-500" />,
                    bg: "bg-blue-50 dark:bg-blue-500/10",
                    border: "border-blue-200 dark:border-blue-500/20"
                };
            case "success":
                return {
                    icon: <CheckCircle2 className="w-5 h-5 text-green-500" />,
                    bg: "bg-green-50 dark:bg-green-500/10",
                    border: "border-green-200 dark:border-green-500/20"
                };
            case "warning":
                return {
                    icon: <AlertTriangle className="w-5 h-5 text-yellow-500" />,
                    bg: "bg-yellow-50 dark:bg-yellow-500/10",
                    border: "border-yellow-200 dark:border-yellow-500/20"
                };
            case "error":
                return {
                    icon: <AlertCircle className="w-5 h-5 text-red-500" />,
                    bg: "bg-red-50 dark:bg-red-500/10",
                    border: "border-red-200 dark:border-red-500/20"
                };
        }
    };

    const handleNotificationClick = (notification: AppNotification) => {
        if (!notification.read) {
            markAsRead(notification.id);
        }
        if (notification.actionLink) {
            window.location.href = notification.actionLink;
        }
    };

    return (
        <div className="relative" ref={dropdownRef}>
            {/* Bell Button */}
            <button
                onClick={() => setIsOpen(!isOpen)}
                className="relative p-2 rounded-lg hover:bg-gray-100 dark:hover:bg-slate-800 transition-colors"
                aria-label="Notifications"
            >
                <Bell className="w-5 h-5 text-gray-700 dark:text-gray-300" />
                {unreadCount > 0 && (
                    <span className="absolute top-1.5 right-1.5 flex h-4 w-4 items-center justify-center rounded-full bg-red-500 text-[10px] font-bold text-white shadow-sm ring-2 ring-white dark:ring-slate-900 border-none">
                        {unreadCount > 99 ? '99+' : unreadCount}
                    </span>
                )}
            </button>

            {/* Dropdown Menu */}
            {isOpen && (
                <div className="absolute right-0 mt-2 w-80 sm:w-96 bg-white dark:bg-slate-900 rounded-xl shadow-[0_8px_30px_rgb(0,0,0,0.12)] border border-gray-100 dark:border-slate-800 overflow-hidden z-50 animate-in fade-in zoom-in duration-200">

                    {/* Header */}
                    <div className="px-4 py-3 border-b border-gray-100 dark:border-slate-800 flex items-center justify-between bg-gray-50/50 dark:bg-slate-900/50">
                        <h3 className="font-semibold text-gray-900 dark:text-white">Notifications</h3>
                        {unreadCount > 0 && (
                            <button
                                onClick={(e: React.MouseEvent) => { e.stopPropagation(); markAllAsRead(); }}
                                className="text-xs font-medium text-blue-600 dark:text-blue-400 hover:text-blue-800 dark:hover:text-blue-300 transition-colors flex items-center gap-1"
                            >
                                <Check className="w-3.5 h-3.5" />
                                Mark all read
                            </button>
                        )}
                    </div>

                    {/* Tabs */}
                    <div className="flex border-b border-gray-100 dark:border-slate-800">
                        <button
                            onClick={() => setActiveTab("all")}
                            className={`flex-1 py-2.5 text-sm font-medium transition-colors relative ${activeTab === "all"
                                ? "text-blue-600 dark:text-blue-400"
                                : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                                }`}
                        >
                            All
                            {activeTab === "all" && (
                                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-blue-600 dark:bg-blue-400" />
                            )}
                        </button>
                        <button
                            onClick={() => setActiveTab("unread")}
                            className={`flex-1 py-2.5 text-sm font-medium transition-colors relative flex items-center justify-center gap-2 ${activeTab === "unread"
                                ? "text-blue-600 dark:text-blue-400"
                                : "text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200"
                                }`}
                        >
                            Unread
                            {unreadCount > 0 && (
                                <span className={`px-1.5 py-0.5 rounded-full text-[10px] font-bold ${activeTab === "unread"
                                    ? "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400"
                                    : "bg-gray-100 text-gray-600 dark:bg-slate-800 dark:text-gray-400"
                                    }`}>
                                    {unreadCount}
                                </span>
                            )}
                            {activeTab === "unread" && (
                                <div className="absolute bottom-0 left-0 right-0 h-0.5 bg-blue-600 dark:bg-blue-400" />
                            )}
                        </button>
                    </div>

                    {/* Notifications List */}
                    <div className="max-h-[min(500px,calc(100vh-120px))] overflow-y-auto overflow-x-hidden scrollbar-thin scrollbar-thumb-gray-200 dark:scrollbar-thumb-slate-700">
                        {filteredNotifications.length > 0 ? (
                            <div className="divide-y divide-gray-50 dark:divide-slate-800/50">
                                {filteredNotifications.map((notification) => {
                                    const styles = getTypeStyles(notification.type);
                                    return (
                                        <div
                                            key={notification.id}
                                            onClick={() => handleNotificationClick(notification)}
                                            className={`group relative p-4 hover:bg-gray-50 dark:hover:bg-slate-800/50 transition-colors cursor-pointer flex gap-4 ${!notification.read ? "bg-blue-50/30 dark:bg-blue-900/10" : ""
                                                }`}
                                        >
                                            {/* Unread dot indicator */}
                                            {!notification.read && (
                                                <div className="absolute left-1.5 top-1/2 -translate-y-1/2 w-1.5 h-1.5 rounded-full bg-blue-500" />
                                            )}

                                            <div className={`mt-0.5 shrink-0 flex items-center justify-center w-10 h-10 rounded-full ${styles.bg} ${styles.border} border`}>
                                                {styles.icon}
                                            </div>

                                            <div className="flex-1 min-w-0 pr-6">
                                                <div className="flex justify-between items-start mb-1">
                                                    <p className={`text-sm font-medium truncate pr-2 ${!notification.read ? "text-gray-900 dark:text-white" : "text-gray-700 dark:text-gray-200"
                                                        }`}>
                                                        {notification.title}
                                                    </p>
                                                    <span className="text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap shrink-0">
                                                        {formatDistanceToNow(notification.createdAt)}
                                                    </span>
                                                </div>
                                                <p className={`text-xs line-clamp-2 ${!notification.read ? "text-gray-600 dark:text-gray-300" : "text-gray-500 dark:text-gray-400"
                                                    }`}>
                                                    {notification.message}
                                                </p>

                                                {notification.actionText && (
                                                    <div className="mt-2 flex items-center text-xs font-medium text-blue-600 dark:text-blue-400 hover:text-blue-700 dark:hover:text-blue-300">
                                                        {notification.actionText}
                                                        <ExternalLink className="w-3 h-3 ml-1" />
                                                    </div>
                                                )}
                                            </div>

                                            {/* Hover Actions */}
                                            <div className="absolute right-3 top-4 opacity-0 group-hover:opacity-100 transition-opacity flex flex-col gap-2">
                                                <button
                                                    onClick={(e: React.MouseEvent) => { e.stopPropagation(); removeNotification(notification.id); }}
                                                    className="p-1 text-gray-400 hover:text-red-500 rounded-md hover:bg-gray-100 dark:hover:bg-slate-700 transition-colors bg-white dark:bg-slate-800 shadow-sm"
                                                    title="Remove notification"
                                                >
                                                    <X className="w-4 h-4" />
                                                </button>
                                            </div>
                                        </div>
                                    );
                                })}
                            </div>
                        ) : (
                            <div className="flex flex-col items-center justify-center py-12 px-4 text-center">
                                <div className="w-16 h-16 bg-gray-50 dark:bg-slate-800 rounded-full flex items-center justify-center mb-4">
                                    <Bell className="w-8 h-8 text-gray-300 dark:text-slate-600" />
                                </div>
                                <h4 className="text-gray-900 dark:text-white font-medium mb-1">
                                    All caught up!
                                </h4>
                                <p className="text-sm text-gray-500 dark:text-gray-400 max-w-[200px]">
                                    {activeTab === 'unread'
                                        ? "You don't have any unread notifications."
                                        : "When you receive new alerts, they will appear here."}
                                </p>
                            </div>
                        )}
                    </div>

                    {/* Footer */}
                    {notifications.length > 0 && (
                        <div className="p-2 border-t border-gray-100 dark:border-slate-800 bg-gray-50/50 dark:bg-slate-900/50">
                            <button
                                onClick={(e: React.MouseEvent) => { e.stopPropagation(); clearAll(); }}
                                className="w-full py-2 text-xs font-medium text-gray-500 hover:text-red-500 dark:text-gray-400 dark:hover:text-red-400 transition-colors flex items-center justify-center gap-1.5 rounded-lg hover:bg-red-50 dark:hover:bg-red-500/10"
                            >
                                <Trash2 className="w-3.5 h-3.5" />
                                Clear all notifications
                            </button>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}

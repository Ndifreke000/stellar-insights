'use client';

import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Bell,
  Settings,
  Filter,
  BarChart3,
} from 'lucide-react';

import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TooltipProvider } from '@/components/ui/tooltip';
import { NotificationListView } from './NotificationListView';
import SettingsTabView from './SettingsTabView';
import FiltersTabView from './FiltersTabView';
import NotificationHeader from './NotificationHeader';
import AnalyticsView from './AnalyticsView';
import useEnhancedNotificationCenter from './useEnhancedNotificationCenter';

interface EnhancedNotificationCenterProps {
  isOpen: boolean;
  onClose: () => void;
}

export const EnhancedNotificationCenter: React.FC<EnhancedNotificationCenterProps> = ({
  isOpen,
  onClose,
}) => {
  const [activeTab, setActiveTab] = useState('notifications');
  const {
    updatePreferences, preferences, selectedFilters, setSelectedFilters, setShowFilters,
    ...rest
  } = useEnhancedNotificationCenter();

  return (
    <TooltipProvider>
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
            transition={{ duration: 0.2 }}
            className="fixed inset-0 z-50 flex items-start justify-end p-4"
          >
            {/* Backdrop */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              className="absolute inset-0 bg-black/20 backdrop-blur-sm"
              onClick={onClose}
            />

            {/* Notification Panel */}
            <motion.div
              initial={{ x: 100, opacity: 0 }}
              animate={{ x: 0, opacity: 1 }}
              exit={{ x: 100, opacity: 0 }}
              transition={{ type: 'spring', damping: 25 }}
              className="relative w-full max-w-2xl h-[80vh] bg-white dark:bg-slate-900 rounded-2xl shadow-2xl border border-gray-200 dark:border-slate-700 overflow-hidden flex flex-col"
            >
              {/* Header */}
              <NotificationHeader unreadCount={rest.unreadCount}
                markAllAsRead={rest.markAllAsRead}
                isWebSocketConnected={rest.isWebSocketConnected}
                filteredNotifications={rest.filteredNotifications}
                showFilters={rest.showFilters}
                setShowFilters={setShowFilters} onClose={onClose} />

              {/* Main Content */}
              <div className="flex-1 overflow-hidden">
                <Tabs value={activeTab} onValueChange={setActiveTab} className="h-full flex flex-col">
                  <TabsList className="grid w-full grid-cols-4 mx-6 mt-4">
                    <TabsTrigger value="notifications" className="flex items-center gap-2">
                      <Bell className="h-4 w-4" />
                      Notifications
                    </TabsTrigger>
                    <TabsTrigger value="analytics" className="flex items-center gap-2">
                      <BarChart3 className="h-4 w-4" />
                      Analytics
                    </TabsTrigger>
                    <TabsTrigger value="filters" className="flex items-center gap-2">
                      <Filter className="h-4 w-4" />
                      Filters
                    </TabsTrigger>
                    <TabsTrigger value="settings" className="flex items-center gap-2">
                      <Settings className="h-4 w-4" />
                      Settings
                    </TabsTrigger>
                  </TabsList>

                  {/* Notifications Tab */}
                  <TabsContent value="notifications" className="flex-1 overflow-hidden m-0">
                    <NotificationListView {...rest} />
                  </TabsContent>

                  {/* Analytics Tab */}
                  <TabsContent value="analytics" className="flex-1 overflow-hidden m-0 p-6">
                    <AnalyticsView />
                  </TabsContent>

                  {/* Filters Tab */}
                  <TabsContent value="filters" className="flex-1 overflow-hidden m-0 p-6">
                    <FiltersTabView selectedFilters={selectedFilters} setSelectedFilters={setSelectedFilters} setShowFilters={setShowFilters} />
                  </TabsContent>

                  {/* Settings Tab */}
                  <TabsContent value="settings" className="flex-1 overflow-hidden m-0 p-6">
                    <SettingsTabView updatePreferences={updatePreferences} preferences={preferences} />
                  </TabsContent>
                </Tabs>
              </div>
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </TooltipProvider>
  );
};

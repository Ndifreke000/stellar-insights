'use client';

import React, { useState } from 'react';
import { motion } from 'framer-motion';
import { 
  Settings, 
  Bell, 
  BellOff, 
  Volume2, 
  VolumeX, 
  Monitor,
  Smartphone,
  Clock,
  Save,
  RotateCcw
} from 'lucide-react';
import { NotificationPreferences as NotificationPreferencesType } from '@/types/notifications';
import { useNotifications } from '@/contexts/NotificationContext';

interface NotificationPreferencesProps {
  isOpen: boolean;
  onClose: () => void;
}

export const NotificationPreferences: React.FC<NotificationPreferencesProps> = ({ 
  isOpen, 
  onClose 
}) => {
  const { preferences, updatePreferences } = useNotifications();
  const [localPreferences, setLocalPreferences] = useState<NotificationPreferencesType>(preferences);
  const [hasChanges, setHasChanges] = useState(false);

  const handlePreferenceChange = (key: keyof NotificationPreferencesType, value: any) => {
    const newPreferences = { ...localPreferences, [key]: value };
    setLocalPreferences(newPreferences);
    setHasChanges(JSON.stringify(newPreferences) !== JSON.stringify(preferences));
  };

  const handleSoundChange = (key: keyof NotificationPreferencesType['sound'], value: any) => {
    const newSound = { ...localPreferences.sound, [key]: value };
    handlePreferenceChange('sound', newSound);
  };

  const handleCategoryChange = (category: keyof NotificationPreferencesType['categories'], enabled: boolean) => {
    const newCategories = { ...localPreferences.categories, [category]: enabled };
    handlePreferenceChange('categories', newCategories);
  };

  const handleSave = () => {
    updatePreferences(localPreferences);
    setHasChanges(false);
  };

  const handleReset = () => {
    setLocalPreferences(preferences);
    setHasChanges(false);
  };

  const testSound = () => {
    // Create a simple test sound
    if (typeof window !== 'undefined' && localPreferences.sound.enabled) {
      const audioContext = new (window.AudioContext || (window as any).webkitAudioContext)();
      const oscillator = audioContext.createOscillator();
      const gainNode = audioContext.createGain();
      
      oscillator.connect(gainNode);
      gainNode.connect(audioContext.destination);
      
      oscillator.frequency.setValueAtTime(800, audioContext.currentTime);
      oscillator.type = 'sine';
      
      gainNode.gain.setValueAtTime(0, audioContext.currentTime);
      gainNode.gain.linearRampToValueAtTime(localPreferences.sound.volume * 0.1, audioContext.currentTime + 0.01);
      gainNode.gain.exponentialRampToValueAtTime(0.001, audioContext.currentTime + 0.2);
      
      oscillator.start(audioContext.currentTime);
      oscillator.stop(audioContext.currentTime + 0.2);
    }
  };

  if (!isOpen) return null;

  return (
    <motion.div
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      exit={{ opacity: 0 }}
      className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <motion.div
        initial={{ scale: 0.9, opacity: 0 }}
        animate={{ scale: 1, opacity: 1 }}
        exit={{ scale: 0.9, opacity: 0 }}
        className="bg-white dark:bg-gray-900 rounded-xl shadow-xl max-w-2xl w-full max-h-[90vh] overflow-y-auto"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="border-b border-gray-200 dark:border-gray-700 p-6">
          <div className="flex items-center justify-between">
            <div className="flex items-center space-x-3">
              <Settings className="h-6 w-6 text-gray-600 dark:text-gray-400" />
              <h2 className="text-xl font-semibold text-gray-900 dark:text-white">
                Notification Preferences
              </h2>
            </div>
            <button
              onClick={onClose}
              className="p-2 hover:bg-gray-100 dark:hover:bg-gray-800 rounded-lg transition-colors"
            >
              ×
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-6 space-y-8">
          {/* General Settings */}
          <div>
            <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4 flex items-center">
              <Bell className="h-5 w-5 mr-2" />
              General Settings
            </h3>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                    Enable Notifications
                  </label>
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    Turn on/off all notifications
                  </p>
                </div>
                <button
                  onClick={() => handlePreferenceChange('enabled', !localPreferences.enabled)}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    localPreferences.enabled ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700'
                  }`}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      localPreferences.enabled ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                    Desktop Notifications
                  </label>
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    Show notifications outside the browser
                  </p>
                </div>
                <button
                  onClick={() => handlePreferenceChange('showOnDesktop', !localPreferences.showOnDesktop)}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    localPreferences.showOnDesktop ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700'
                  }`}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      localPreferences.showOnDesktop ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
              </div>

              <div className="flex items-center justify-between">
                <div>
                  <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                    Auto Hide
                  </label>
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    Automatically dismiss notifications
                  </p>
                </div>
                <button
                  onClick={() => handlePreferenceChange('autoHide', !localPreferences.autoHide)}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    localPreferences.autoHide ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700'
                  }`}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      localPreferences.autoHide ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
              </div>

              {localPreferences.autoHide && (
                <div>
                  <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                    Auto Hide Delay (seconds)
                  </label>
                  <input
                    type="range"
                    min="1"
                    max="30"
                    value={localPreferences.autoHideDelay / 1000}
                    onChange={(e) => handlePreferenceChange('autoHideDelay', parseInt(e.target.value) * 1000)}
                    className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer"
                  />
                  <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1">
                    <span>1s</span>
                    <span>{localPreferences.autoHideDelay / 1000}s</span>
                    <span>30s</span>
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Sound Settings */}
          <div>
            <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4 flex items-center">
              <Volume2 className="h-5 w-5 mr-2" />
              Sound Settings
            </h3>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <div>
                  <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                    Enable Sound
                  </label>
                  <p className="text-sm text-gray-500 dark:text-gray-400">
                    Play sound for notifications
                  </p>
                </div>
                <button
                  onClick={() => handleSoundChange('enabled', !localPreferences.sound.enabled)}
                  className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                    localPreferences.sound.enabled ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700'
                  }`}
                >
                  <span
                    className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                      localPreferences.sound.enabled ? 'translate-x-6' : 'translate-x-1'
                    }`}
                  />
                </button>
              </div>

              {localPreferences.sound.enabled && (
                <>
                  <div>
                    <div className="flex items-center justify-between mb-2">
                      <label className="text-sm font-medium text-gray-700 dark:text-gray-300">
                        Volume
                      </label>
                      <button
                        onClick={testSound}
                        className="px-3 py-1 text-xs bg-blue-500 text-white rounded-md hover:bg-blue-600 transition-colors"
                      >
                        Test Sound
                      </button>
                    </div>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.1"
                      value={localPreferences.sound.volume}
                      onChange={(e) => handleSoundChange('volume', parseFloat(e.target.value))}
                      className="w-full h-2 bg-gray-200 dark:bg-gray-700 rounded-lg appearance-none cursor-pointer"
                    />
                    <div className="flex justify-between text-xs text-gray-500 dark:text-gray-400 mt-1">
                      <span>0%</span>
                      <span>{Math.round(localPreferences.sound.volume * 100)}%</span>
                      <span>100%</span>
                    </div>
                  </div>

                  <div>
                    <label className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2">
                      Sound Type
                    </label>
                    <select
                      value={localPreferences.sound.soundType}
                      onChange={(e) => handleSoundChange('soundType', e.target.value)}
                      className="w-full px-3 py-2 border border-gray-300 dark:border-gray-600 rounded-lg bg-white dark:bg-gray-800 text-gray-900 dark:text-white"
                    >
                      <option value="default">Default</option>
                      <option value="subtle">Subtle</option>
                      <option value="alert">Alert</option>
                      <option value="critical">Critical</option>
                    </select>
                  </div>
                </>
              )}
            </div>
          </div>

          {/* Category Settings */}
          <div>
            <h3 className="text-lg font-medium text-gray-900 dark:text-white mb-4 flex items-center">
              <Monitor className="h-5 w-5 mr-2" />
              Notification Categories
            </h3>
            <div className="space-y-4">
              {Object.entries(localPreferences.categories).map(([category, enabled]) => (
                <div key={category} className="flex items-center justify-between">
                  <div>
                    <label className="text-sm font-medium text-gray-700 dark:text-gray-300 capitalize">
                      {category}
                    </label>
                    <p className="text-sm text-gray-500 dark:text-gray-400">
                      {category === 'payments' && 'Failed payment notifications'}
                      {category === 'liquidity' && 'Low liquidity alerts'}
                      {category === 'snapshots' && 'New snapshot notifications'}
                      {category === 'system' && 'System alerts and updates'}
                    </p>
                  </div>
                  <button
                    onClick={() => handleCategoryChange(category as keyof NotificationPreferencesType['categories'], !enabled)}
                    className={`relative inline-flex h-6 w-11 items-center rounded-full transition-colors ${
                      enabled ? 'bg-blue-600' : 'bg-gray-200 dark:bg-gray-700'
                    }`}
                  >
                    <span
                      className={`inline-block h-4 w-4 transform rounded-full bg-white transition-transform ${
                        enabled ? 'translate-x-6' : 'translate-x-1'
                      }`}
                    />
                  </button>
                </div>
              ))}
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="border-t border-gray-200 dark:border-gray-700 p-6">
          <div className="flex justify-between">
            <button
              onClick={handleReset}
              disabled={!hasChanges}
              className="flex items-center px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
            >
              <RotateCcw className="h-4 w-4 mr-2" />
              Reset Changes
            </button>
            <div className="flex space-x-3">
              <button
                onClick={onClose}
                className="px-4 py-2 text-sm text-gray-600 dark:text-gray-400 hover:text-gray-800 dark:hover:text-gray-200 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={handleSave}
                disabled={!hasChanges}
                className="flex items-center px-4 py-2 text-sm bg-blue-500 text-white rounded-lg hover:bg-blue-600 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
              >
                <Save className="h-4 w-4 mr-2" />
                Save Changes
              </button>
            </div>
          </div>
        </div>
      </motion.div>
    </motion.div>
  );
};
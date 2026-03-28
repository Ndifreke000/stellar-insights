import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Switch } from '@/components/ui/switch';
import { Label } from '@/components/ui/label';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { CATEGORY_ICONS } from './util';
import useEnhancedNotificationCenter from './useEnhancedNotificationCenter';
import React from 'react';
import { NotificationPreferences } from '@/types/notifications';

const SettingsTabView = ({ updatePreferences, preferences }: {
  updatePreferences: (preferences: Partial<NotificationPreferences>) => void;
  preferences: NotificationPreferences;
}) => {
  return (
    <div className="max-w-2xl space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Notification Preferences</CardTitle>
          <CardDescription>
            Configure how you receive notifications
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Enable Notifications */}
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-sm font-medium">
                Enable Notifications
              </Label>
              <p className="text-sm text-gray-600">
                Turn notifications on or off globally
              </p>
            </div>
            <Switch
              checked={preferences.enabled}
              onCheckedChange={(enabled) => updatePreferences({ enabled })}
            />
          </div>

          {/* Desktop Notifications */}
          <div className="flex items-center justify-between">
            <div>
              <Label className="text-sm font-medium">
                Desktop Notifications
              </Label>
              <p className="text-sm text-gray-600">
                Show notifications on your desktop
              </p>
            </div>
            <Switch
              checked={preferences.showOnDesktop}
              onCheckedChange={(showOnDesktop) =>
                updatePreferences({ showOnDesktop })
              }
            />
          </div>

          {/* Sound */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-sm font-medium">Sound</Label>
                <p className="text-sm text-gray-600">
                  Play sound for notifications
                </p>
              </div>
              <Switch
                checked={preferences.sound.enabled}
                onCheckedChange={(enabled) =>
                  updatePreferences({
                    sound: { ...preferences.sound, enabled },
                  })
                }
              />
            </div>

            {preferences.sound.enabled && (
              <div className="space-y-4 pl-4 border-l-2 border-gray-200 dark:border-slate-700">
                <div>
                  <Label className="text-sm font-medium">Volume</Label>
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.1"
                    value={preferences.sound.volume}
                    onChange={(e) =>
                      updatePreferences({
                        sound: {
                          ...preferences.sound,
                          volume: parseFloat(e.target.value),
                        },
                      })
                    }
                    className="w-full mt-2"
                  />
                </div>

                <div>
                  <Label className="text-sm font-medium">Sound Type</Label>
                  <Select
                    value={preferences.sound.soundType}
                    onValueChange={(soundType: any) =>
                      updatePreferences({
                        sound: { ...preferences.sound, soundType },
                      })
                    }
                  >
                    <SelectTrigger className="w-full mt-2">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectItem value="default">Default</SelectItem>
                      <SelectItem value="subtle">Subtle</SelectItem>
                      <SelectItem value="alert">Alert</SelectItem>
                      <SelectItem value="critical">Critical</SelectItem>
                    </SelectContent>
                  </Select>
                </div>
              </div>
            )}
          </div>

          {/* Auto-hide */}
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <div>
                <Label className="text-sm font-medium">Auto-hide</Label>
                <p className="text-sm text-gray-600">
                  Automatically hide notifications after a delay
                </p>
              </div>
              <Switch
                checked={preferences.autoHide}
                onCheckedChange={(autoHide) => updatePreferences({ autoHide })}
              />
            </div>

            {preferences.autoHide && (
              <div className="pl-4 border-l-2 border-gray-200 dark:border-slate-700">
                <Label className="text-sm font-medium">Delay (seconds)</Label>
                <Input
                  type="number"
                  min="1"
                  max="30"
                  value={preferences.autoHideDelay / 1000}
                  onChange={(e) =>
                    updatePreferences({
                      autoHideDelay: parseInt(e.target.value) * 1000,
                    })
                  }
                  className="w-full mt-2"
                />
              </div>
            )}
          </div>

          {/* Categories */}
          <div>
            <Label className="text-sm font-medium mb-4 block">Categories</Label>
            <div className="space-y-3">
              {Object.entries(preferences.categories).map(
                ([category, enabled]) => (
                  <div
                    key={category}
                    className="flex items-center justify-between"
                  >
                    <div className="flex items-center gap-2">
                      {CATEGORY_ICONS[category] &&
                        React.createElement(CATEGORY_ICONS[category], {
                          className: 'h-4 w-4',
                        })}
                      <span className="capitalize">{category}</span>
                    </div>
                    <Switch
                      checked={enabled}
                      onCheckedChange={(categoryEnabled) =>
                        updatePreferences({
                          categories: {
                            ...preferences.categories,
                            [category]: categoryEnabled,
                          },
                        })
                      }
                    />
                  </div>
                ),
              )}
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};

export default SettingsTabView;

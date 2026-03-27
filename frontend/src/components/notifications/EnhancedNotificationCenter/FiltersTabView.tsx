import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Label } from '@/components/ui/label';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import {
  CATEGORY_ICONS,
  NOTIFICATION_ICONS,
  PRIORITY_COLORS,
  TYPE_COLORS,
} from './util';
import React from 'react';
import { Button } from '@/components/ui/button';
import { format } from 'date-fns';
import { NotificationPriority, NotificationType } from '@/types/notifications';
import { Badge } from '@/components/ui/badge';
import { NotificationFilter } from '@/services/notificationService';

const FiltersTabView = ({ selectedFilters, setSelectedFilters, setShowFilters }: {
  selectedFilters: NotificationFilter; setSelectedFilters: React.Dispatch<React.SetStateAction<NotificationFilter>>;
  setShowFilters: React.Dispatch<React.SetStateAction<boolean>>
}) => {

  return (
    <div className="max-w-2xl space-y-6">
      <Card>
        <CardHeader>
          <CardTitle>Filter Options</CardTitle>
          <CardDescription>
            Customize which notifications you want to see
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-6">
          {/* Date Range */}
          <div>
            <Label className="text-sm font-medium">Date Range</Label>
            <div className="grid grid-cols-2 gap-4 mt-2">
              <Input
                type="date"
                value={
                  selectedFilters.dateRange?.start
                    ? format(selectedFilters.dateRange.start, 'yyyy-MM-dd')
                    : ''
                }
                onChange={(e) => {
                  const start = e.target.value
                    ? new Date(e.target.value)
                    : undefined;
                  setSelectedFilters((prev) => ({
                    ...prev,
                    dateRange: prev.dateRange
                      ? { ...prev.dateRange, start }
                      : start
                        ? { start, end: new Date() }
                        : undefined,
                  }));
                }}
              />
              <Input
                type="date"
                value={
                  selectedFilters.dateRange?.end
                    ? format(selectedFilters.dateRange.end, 'yyyy-MM-dd')
                    : ''
                }
                onChange={(e) => {
                  const end = e.target.value
                    ? new Date(e.target.value)
                    : undefined;
                  setSelectedFilters((prev) => ({
                    ...prev,
                    dateRange: prev.dateRange
                      ? { ...prev.dateRange, end }
                      : end
                        ? { start: new Date(), end }
                        : undefined,
                  }));
                }}
              />
            </div>
          </div>

          {/* Types */}
          <div>
            <Label className="text-sm font-medium">Types</Label>
            <div className="grid grid-cols-2 gap-2 mt-2">
              {(
                ['success', 'error', 'warning', 'info'] as NotificationType[]
              ).map((type) => {
                const Icon = NOTIFICATION_ICONS[type];
                return (
                  <div key={type} className="flex items-center space-x-2">
                    <Checkbox
                      id={`type-${type}`}
                      checked={selectedFilters.types?.includes(type) || false}
                      onCheckedChange={(checked) => {
                        setSelectedFilters((prev) => ({
                          ...prev,
                          types: checked
                            ? [...(prev.types || []), type]
                            : (prev.types || []).filter((t) => t !== type),
                        }));
                      }}
                    />
                    <Label
                      htmlFor={`type-${type}`}
                      className="flex items-center gap-2"
                    >
                      <Icon className={`h-4 w-4 ${TYPE_COLORS[type]}`} />
                      <span className="capitalize">{type}</span>
                    </Label>
                  </div>
                );
              })}
            </div>
          </div>

          {/* Priorities */}
          <div>
            <Label className="text-sm font-medium">Priorities</Label>
            <div className="grid grid-cols-2 gap-2 mt-2">
              {(
                ['low', 'medium', 'high', 'critical'] as NotificationPriority[]
              ).map((priority) => (
                <div key={priority} className="flex items-center space-x-2">
                  <Checkbox
                    id={`priority-${priority}`}
                    checked={
                      selectedFilters.priorities?.includes(priority) || false
                    }
                    onCheckedChange={(checked) => {
                      setSelectedFilters((prev) => ({
                        ...prev,
                        priorities: checked
                          ? [...(prev.priorities || []), priority]
                          : (prev.priorities || []).filter(
                              (p) => p !== priority,
                            ),
                      }));
                    }}
                  />
                  <Label
                    htmlFor={`priority-${priority}`}
                    className="flex items-center gap-2"
                  >
                    <Badge className={PRIORITY_COLORS[priority]}>
                      {priority}
                    </Badge>
                  </Label>
                </div>
              ))}
            </div>
          </div>

          {/* Categories */}
          <div>
            <Label className="text-sm font-medium">Categories</Label>
            <div className="grid grid-cols-2 gap-2 mt-2">
              {[
                'payments',
                'liquidity',
                'snapshots',
                'system',
                'security',
                'network',
                'maintenance',
                'updates',
              ].map((category) => (
                <div key={category} className="flex items-center space-x-2">
                  <Checkbox
                    id={`category-${category}`}
                    checked={
                      selectedFilters.categories?.includes(category) || false
                    }
                    onCheckedChange={(checked) => {
                      setSelectedFilters((prev) => ({
                        ...prev,
                        categories: checked
                          ? [...(prev.categories || []), category]
                          : (prev.categories || []).filter(
                              (c) => c !== category,
                            ),
                      }));
                    }}
                  />
                  <Label
                    htmlFor={`category-${category}`}
                    className="flex items-center gap-2"
                  >
                    {CATEGORY_ICONS[category] &&
                      React.createElement(CATEGORY_ICONS[category], {
                        className: 'h-4 w-4',
                      })}
                    <span className="capitalize">{category}</span>
                  </Label>
                </div>
              ))}
            </div>
          </div>

          {/* Read Status */}
          <div>
            <Label className="text-sm font-medium">Read Status</Label>
            <div className="flex gap-4 mt-2">
              {['all', 'read', 'unread'].map((status) => (
                <div key={status} className="flex items-center space-x-2">
                  <input
                    type="radio"
                    id={`status-${status}`}
                    name="read-status"
                    checked={
                      selectedFilters.readStatus === status ||
                      (!selectedFilters.readStatus && status === 'all')
                    }
                    onChange={() => {
                      setSelectedFilters((prev) => ({
                        ...prev,
                        readStatus: status as any,
                      }));
                    }}
                  />
                  <Label htmlFor={`status-${status}`} className="capitalize">
                    {status}
                  </Label>
                </div>
              ))}
            </div>
          </div>

          {/* Clear Filters */}
          <div className="flex gap-2">
            <Button variant="outline" onClick={() => setSelectedFilters({})}>
              Clear All Filters
            </Button>
            <Button onClick={() => setShowFilters(false)}>Apply Filters</Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
};

export default FiltersTabView;

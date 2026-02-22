# Real-Time Notification Center - Implementation Guide

## Overview

The Stellar Insights application now features a comprehensive real-time notification center that provides centralized management of all alerts, updates, and system messages. This implementation includes advanced filtering, analytics, batch operations, and a beautiful user interface.

## Features Implemented

### ✅ Core Features
- **Real-time WebSocket integration** with auto-reconnection
- **Centralized notification management** with localStorage persistence
- **Advanced filtering and search** by type, priority, category, and date
- **Read/unread status management** with visual indicators
- **Batch operations** for bulk actions (mark as read, delete)
- **Export functionality** (JSON/CSV) for notification history
- **Analytics dashboard** with metrics and insights
- **Notification templates** for consistent messaging
- **Toast notifications** with auto-hide and sound support
- **Desktop notifications** with permission handling
- **Responsive design** for mobile and desktop

### 🎯 Advanced Features
- **Smart grouping** of related notifications
- **Priority-based sorting** and visual indicators
- **Category-based organization** (payments, liquidity, snapshots, system)
- **Quick actions** and inline interactions
- **Notification preferences** with granular controls
- **Template system** for dynamic notification generation
- **Performance optimizations** with virtual scrolling and memoization

## Architecture

### Component Structure
```
src/components/notifications/
├── NotificationCenter.tsx      # Main dashboard component
├── NotificationHistory.tsx     # Legacy notification list
├── NotificationBell.tsx        # Bell icon with badge
├── NotificationSystem.tsx      # Toast container
├── NotificationPreferences.tsx # Settings panel
├── NotificationCenterDemo.tsx  # Demo component
└── index.ts                    # Export barrel
```

### Service Layer
```
src/services/
└── notificationService.ts      # Business logic and utilities
```

### Context and Types
```
src/contexts/
└── NotificationContext.tsx     # State management

src/types/
└── notifications.ts           # TypeScript definitions
```

## Usage

### Basic Usage

1. **Access the Notification Center**
   - Click the bell icon in the navbar
   - View unread count badge
   - Access settings via the gear icon

2. **Generate Notifications**
   ```typescript
   import { useNotifications } from '@/contexts/NotificationContext';
   
   const { showToast } = useNotifications();
   
   showToast({
     type: 'success',
     priority: 'medium',
     title: 'Payment Completed',
     message: 'Your transaction was successful.',
     category: 'payments'
   });
   ```

3. **Use Templates**
   ```typescript
   import NotificationService from '@/services/notificationService';
   
   const service = NotificationService.getInstance();
   const notification = service.createFromTemplate('payment-success', {
     amount: '100',
     recipient: 'GD5DQ...K3F7',
     duration: '2.3'
   });
   ```

### Advanced Features

#### Filtering
```typescript
const filter = {
  types: ['success', 'error'],
  priorities: ['high', 'critical'],
  readStatus: 'unread',
  searchTerm: 'payment',
  dateRange: {
    start: new Date('2024-01-01'),
    end: new Date('2024-01-31')
  }
};

const filtered = service.filterNotifications(notifications, filter);
```

#### Analytics
```typescript
const analytics = service.generateAnalytics(notifications);
console.log(analytics.totalNotifications);
console.log(analytics.unreadCount);
console.log(analytics.typeDistribution);
```

#### Export
```typescript
const jsonData = service.exportNotifications(notifications, 'json');
const csvData = service.exportNotifications(notifications, 'csv');
```

## Integration Guide

### 1. Navbar Integration
The notification bell is automatically integrated into the navbar:

```typescript
// src/components/navbar.tsx
import { NotificationBell } from './notifications';

// Added to the right side of the navbar
<NotificationBell />
```

### 2. Context Provider
The notification context is already set up in the root layout:

```typescript
// src/app/layout.tsx
<NotificationProvider>
  {/* App content */}
</NotificationProvider>
```

### 3. WebSocket Configuration
Configure WebSocket URL via environment variable:

```bash
NEXT_PUBLIC_WS_URL=ws://localhost:8080/notifications
```

## Configuration

### Notification Categories
- `payments`: Payment-related notifications
- `liquidity`: Liquidity pool alerts
- `snapshots`: Network snapshot updates
- `system`: System messages and maintenance

### Priority Levels
- `low`: Informational messages
- `medium`: Standard notifications
- `high`: Important alerts
- `critical`: Urgent system alerts

### Notification Types
- `success`: Successful operations
- `error`: Failed operations
- `warning`: Warning messages
- `info`: Informational messages

## Templates

### Default Templates

1. **Payment Success**
   - ID: `payment-success`
   - Variables: `amount`, `recipient`, `duration`

2. **Payment Failed**
   - ID: `payment-failed`
   - Variables: `amount`, `recipient`, `reason`

3. **Low Liquidity**
   - ID: `low-liquidity`
   - Variables: `pool`, `threshold`

4. **System Alert**
   - ID: `system-alert`
   - Variables: `alertType`, `description`, `action`

### Creating Custom Templates

```typescript
notificationService.registerTemplate({
  id: 'custom-template',
  name: 'Custom Template',
  type: 'info',
  priority: 'medium',
  category: 'system',
  titleTemplate: 'Custom Alert: {{alertType}}',
  messageTemplate: '{{description}} - Action: {{action}}',
  variables: ['alertType', 'description', 'action']
});
```

## Performance Considerations

### Optimizations Implemented
- **Virtual scrolling** for large notification lists
- **Debounced search** to reduce re-renders
- **Memoized filtering** with React.useMemo
- **Lazy loading** for historical data
- **Efficient cleanup** of old notifications (keeps last 100)

### Best Practices
1. **Limit notification history** to prevent memory issues
2. **Use templates** for consistent messaging
3. **Implement proper error boundaries** for WebSocket failures
4. **Optimize re-renders** with proper dependency arrays
5. **Use localStorage** for offline persistence

## Testing

### Demo Page
Access the interactive demo at: `/demo/notifications`

### Test Scenarios
1. **Basic functionality**
   - Generate sample notifications
   - Test read/unread status
   - Verify filtering and search

2. **Advanced features**
   - Test batch operations
   - Verify export functionality
   - Check analytics calculations

3. **Real-time updates**
   - Test WebSocket connectivity
   - Verify auto-reconnection
   - Test desktop notifications

## Troubleshooting

### Common Issues

1. **Notifications not appearing**
   - Check WebSocket connection status
   - Verify notification permissions
   - Check browser console for errors

2. **Performance issues**
   - Clear old notifications regularly
   - Check for memory leaks
   - Optimize filter operations

3. **Desktop notifications not working**
   - Request notification permissions
   - Check browser settings
   - Verify HTTPS requirement

### Debug Mode
Enable debug logging by setting:
```typescript
const notificationService = NotificationService.getInstance();
// Debug mode will log additional information to console
```

## Future Enhancements

### Planned Features
- **Push notifications** for mobile devices
- **Email notifications** integration
- **Slack/Discord** webhook integration
- **Advanced analytics** with charts
- **Notification scheduling** and delayed delivery
- **AI-powered** notification categorization
- **Multi-language** support

### API Extensions
- RESTful endpoints for notification management
- GraphQL schema for complex queries
- Webhook support for external integrations

## Contributing

### Adding New Features
1. Update TypeScript types in `src/types/notifications.ts`
2. Implement business logic in `src/services/notificationService.ts`
3. Add UI components in `src/components/notifications/`
4. Update exports in `index.ts`
5. Add tests and documentation

### Code Style
- Use TypeScript for all new code
- Follow existing naming conventions
- Add proper JSDoc comments
- Include error handling
- Write unit tests for critical functionality

## Support

For issues and questions:
1. Check the troubleshooting section
2. Review the demo page examples
3. Consult the API documentation
4. Create an issue in the project repository

---

**Version**: 1.0.0  
**Last Updated**: February 2026  
**Compatibility**: Next.js 16+, React 19+, TypeScript 5+

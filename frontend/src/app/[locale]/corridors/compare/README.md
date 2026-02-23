# Corridor Comparison Feature

## Quick Start

Access the comparison tool at `/corridors/compare` or add corridor IDs via URL:

```
/corridors/compare?ids=USDC-XLM,EURC-PHP
```

## Features

### 1. Multi-Corridor Selection (2-4 corridors)
- Add corridors dynamically
- Remove corridors with one click
- Visual pills showing selected corridors
- Maximum 4 corridors for optimal comparison

### 2. Comprehensive Metrics
- Success Rate
- Health Score
- Average Latency
- Liquidity Depth
- 24h Volume
- Average Slippage

### 3. Visual Indicators
- 🏆 Best performer
- ⚠️ Worst performer
- 📈 Above average
- 📉 Below average

### 4. Interactive Charts
- Success Rate Timeline (30 days)
- Volume Comparison (Bar chart)
- Slippage Trends (Line chart)

### 5. Export & Share
- Export to CSV
- Share via URL
- Copy link to clipboard

## Usage Examples

### Compare Two Corridors
```
/corridors/compare?ids=USDC-XLM,EURC-PHP
```

### Compare Three Corridors
```
/corridors/compare?ids=USDC-XLM,EURC-PHP,USDC-NGN
```

### Compare Four Corridors (Maximum)
```
/corridors/compare?ids=USDC-XLM,EURC-PHP,USDC-NGN,XLM-BTC
```

## Component Structure

```
compare/
├── page.tsx                 # Main comparison page
└── README.md               # This file

components/
├── CorridorComparisonTable.tsx      # Detailed metrics table
└── corridors/
    ├── CorridorCompareCharts.tsx    # Chart components
    └── CorridorCompareCards.tsx     # Metric cards
```

## API Integration

The comparison tool fetches data from:
- `GET /api/corridors/:id` - Individual corridor details

Data includes:
- Current metrics
- Historical success rates (30 days)
- Volume trends
- Slippage history
- Latency distribution

## State Management

State is managed through:
1. URL parameters (`ids` query param)
2. Local component state
3. React hooks for data fetching

## Responsive Design

- **Desktop**: Full table view with all columns
- **Tablet**: Horizontal scroll, stacked charts
- **Mobile**: Card view, simplified table

## Performance Optimization

- Lazy loading of chart data
- Memoized calculations
- Efficient re-renders
- Suspense boundaries

## Accessibility

- Keyboard navigation
- Screen reader support
- ARIA labels
- Focus management
- High contrast support

## Testing

Run tests with:
```bash
npm test CorridorComparison
```

Tests cover:
- Component rendering
- Metric calculations
- Export functionality
- URL parsing
- Performance indicators

## Future Enhancements

- [ ] Save comparison presets
- [ ] Email reports
- [ ] PDF export
- [ ] Advanced filtering
- [ ] Custom metrics
- [ ] Comparison alerts
- [ ] Historical comparisons

## Troubleshooting

### Corridors Not Loading
1. Check network connection
2. Verify corridor IDs
3. Check browser console
4. Try refreshing

### Export Issues
1. Check browser download settings
2. Disable popup blockers
3. Try different browser

### Chart Issues
1. Enable JavaScript
2. Check browser compatibility
3. Clear cache

## Support

For issues:
1. Check documentation
2. Search existing issues
3. Create new issue with details
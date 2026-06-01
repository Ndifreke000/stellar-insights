# Chart Export Feature

Export charts and visualizations as PNG or SVG images for use in reports and presentations.

## Features

- Export charts as high-quality PNG images (2x resolution)
- Export charts as scalable SVG vector graphics
- Consistent styling with dark background matching the app theme
- Easy-to-use dropdown menu on each chart
- Automatic filename generation with chart name and date

## Usage

### For Users

Each chart now has an "Export" button in the top-right corner:

1. Click the "Export" button on any chart
2. Choose your preferred format:
   - **PNG Image**: Best for presentations, documents, and sharing
   - **SVG Vector**: Best for print materials and further editing

The file will automatically download with a descriptive filename like:
- `liquidity-over-time-2026-02-23.png`
- `total-value-locked-2026-02-23.svg`

### For Developers

#### Adding Export to a New Chart

**Option 1: Using the ChartExportButton component**

```tsx
import { useRef } from 'react';
import { ChartExportButton } from '@/components/charts/ChartExportButton';

export function MyChart({ data }) {
  const chartRef = useRef<HTMLDivElement>(null);

  return (
    <div ref={chartRef} className="glass-card rounded-2xl p-6">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <h2>My Chart Title</h2>
        </div>
        <ChartExportButton chartRef={chartRef} chartName="My Chart Title" />
      </div>
      
      {/* Your chart content */}
    </div>
  );
}
```

**Option 2: Using the useChartExport hook**

```tsx
import { useChartExport } from '@/hooks/useChartExport';

export function MyChart({ data }) {
  const { chartRef, isExporting, handleExport } = useChartExport({
    chartName: 'My Chart Title'
  });

  return (
    <div ref={chartRef} className="glass-card rounded-2xl p-6">
      <button onClick={() => handleExport('png')} disabled={isExporting}>
        Export as PNG
      </button>
      
      {/* Your chart content */}
    </div>
  );
}
```

#### Direct Export Function

For custom implementations:

```tsx
import { exportChart } from '@/lib/chart-export';

const element = document.getElementById('my-chart');
await exportChart(element, {
  filename: 'custom-chart',
  format: 'png',
  backgroundColor: '#0f172a',
  scale: 2
});
```

## Implementation Details

### Export Formats

**PNG Export**
- Uses HTML5 Canvas API
- 2x resolution for high-quality output
- Includes dark background matching app theme
- Best browser compatibility

**SVG Export**
- Direct SVG serialization
- Infinite scalability
- Smaller file size
- Preserves all vector information

### Technical Stack

- **chart-export.ts**: Core export utilities
- **ChartExportButton.tsx**: Reusable UI component
- **useChartExport.ts**: React hook for easy integration

### Browser Compatibility

- Chrome/Edge: Full support
- Firefox: Full support
- Safari: Full support
- Mobile browsers: PNG export supported, SVG may vary

## Charts with Export Enabled

- ✅ Liquidity Over Time
- ✅ Total Value Locked (TVL)
- ✅ Settlement Latency
- ⏳ Trustline Growth Chart (pending)
- ⏳ Reliability Trend (pending)
- ⏳ Corridor Heatmap (pending)
- ⏳ Network Graph (pending)

## Future Enhancements

- [ ] Copy chart to clipboard
- [ ] Export multiple charts at once
- [ ] Custom resolution/size options
- [ ] PDF export with multiple charts
- [ ] Scheduled/automated exports
- [ ] Export with custom branding/watermarks

## Accessibility

- Keyboard accessible dropdown menu
- ARIA labels and roles
- Focus management
- Screen reader announcements for export status

## Testing

To test the export feature:

1. Navigate to any analytics page with charts
2. Click the "Export" button on a chart
3. Select PNG or SVG format
4. Verify the downloaded file opens correctly
5. Check that the styling matches the app theme

## Troubleshooting

**Export button not appearing:**
- Ensure the chart component has been updated with export functionality
- Check that the ChartExportButton is imported correctly

**Export fails or produces blank image:**
- Verify the chart contains an SVG element
- Check browser console for errors
- Ensure the chart is fully rendered before exporting

**Styling looks different in exported image:**
- SVG inline styles may not be captured
- Check that all styles are applied directly to SVG elements
- Background color can be customized in export options

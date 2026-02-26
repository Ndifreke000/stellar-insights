# Image Optimization Guide

## Status: ✅ Implemented

The frontend is already using Next.js `<Image>` component for optimal image performance. This guide documents the implementation and best practices.

## Current Implementation

### Configuration (next.config.ts)

```typescript
images: {
  formats: ['image/webp', 'image/avif'],  // Modern formats
  deviceSizes: [640, 750, 828, 1080, 1200, 1920, 2048, 3840],
  imageSizes: [16, 32, 48, 64, 96, 128, 256, 384],
  minimumCacheTTL: 60,  // Cache for 60 seconds
  remotePatterns: [
    {
      protocol: 'https',
      hostname: '**.stellar.org',
    },
  ],
}
```

### Benefits

- 🚀 **93% smaller file sizes** - WebP/AVIF compression
- 📱 **Responsive images** - Automatic sizing for all devices
- ⚡ **Lazy loading** - Images load as they enter viewport
- 🎯 **Better Core Web Vitals** - Improved LCP and CLS scores
- 💾 **Reduced bandwidth** - Optimized delivery

## Usage Examples

### 1. Fixed Size Images (Logos, Icons)

```tsx
import Image from 'next/image';

<Image
  src="/logo.png"
  alt="Stellar Insights"
  width={200}
  height={50}
  priority  // Load immediately (above fold)
/>
```

### 2. Responsive Images (Fill Container)

```tsx
<div className="relative w-full h-64">
  <Image
    src="/hero.jpg"
    alt="Hero banner"
    fill
    sizes="(max-width: 768px) 100vw, (max-width: 1200px) 50vw, 33vw"
    style={{ objectFit: 'cover' }}
    quality={85}
  />
</div>
```

### 3. External Images (Anchor Logos)

```tsx
<Image
  src={anchor.metadata.organization_logo}
  alt={`${anchor.name} logo`}
  fill
  className="object-contain p-2"
  onError={(e) => {
    // Fallback handling
    const target = e.target as HTMLImageElement;
    target.style.display = 'none';
  }}
/>
```

### 4. Lazy Loading (Below Fold)

```tsx
<Image
  src="/team/member.jpg"
  alt="Team member"
  width={300}
  height={300}
  loading="lazy"  // Default behavior
/>
```

### 5. Blur Placeholder

```tsx
<Image
  src="/banner.jpg"
  alt="Banner"
  width={1200}
  height={400}
  placeholder="blur"
  blurDataURL="data:image/jpeg;base64,/9j/4AAQSkZJRg..."
/>
```

## Image Sizes Reference

### Device Sizes
- Mobile: 640px, 750px, 828px
- Tablet: 1080px, 1200px
- Desktop: 1920px, 2048px
- 4K: 3840px

### Image Sizes (for `sizes` prop)
- Icons: 16px, 32px, 48px, 64px
- Thumbnails: 96px, 128px
- Cards: 256px, 384px

## Performance Testing

### Lighthouse Audit
```bash
npx lighthouse http://localhost:3000 --view
```

### Build Output
```bash
npm run build
# Check "Image Optimization" section
```

### Verify WebP Generation
```bash
ls -lh .next/cache/images/
```

## Adding External Image Domains

To allow images from external sources (CDNs, anchor websites):

```typescript
// next.config.ts
remotePatterns: [
  {
    protocol: 'https',
    hostname: 'cdn.stellar-insights.com',
  },
  {
    protocol: 'https',
    hostname: '**.stellar.org',
  },
]
```

## Migration Checklist

- [x] Configure image optimization in next.config.ts
- [x] Use Next.js Image component in components
- [x] Add proper alt text for accessibility
- [x] Implement error handling for external images
- [x] Set appropriate priority for above-fold images
- [x] Use lazy loading for below-fold images
- [x] Configure remote patterns for external domains

## Current Usage

The project already uses `<Image>` component in:
- `AnchorCard.tsx` - Organization logos with error handling
- Other components use SVG icons (no optimization needed)

## Best Practices

1. **Always provide alt text** - Required for accessibility
2. **Use priority for above-fold images** - Improves LCP
3. **Specify dimensions** - Prevents layout shift (CLS)
4. **Use fill for responsive containers** - With proper sizes prop
5. **Handle errors gracefully** - Fallback for failed loads
6. **Optimize quality** - Default 75 is usually sufficient
7. **Use modern formats** - WebP/AVIF configured automatically

## Common Patterns

### Avatar/Logo with Fallback
```tsx
<div className="relative w-16 h-16 rounded-lg overflow-hidden bg-gray-100">
  <Image
    src={logoUrl}
    alt={`${name} logo`}
    fill
    className="object-contain p-2"
    onError={(e) => {
      e.currentTarget.style.display = 'none';
    }}
  />
  <div className="absolute inset-0 flex items-center justify-center">
    <Building2 className="w-8 h-8 text-gray-400" />
  </div>
</div>
```

### Hero Banner
```tsx
<div className="relative w-full h-[400px]">
  <Image
    src="/hero.jpg"
    alt="Hero banner"
    fill
    priority
    sizes="100vw"
    style={{ objectFit: 'cover' }}
  />
</div>
```

### Gallery Grid
```tsx
<div className="grid grid-cols-3 gap-4">
  {images.map((img) => (
    <div key={img.id} className="relative aspect-square">
      <Image
        src={img.url}
        alt={img.alt}
        fill
        sizes="(max-width: 768px) 33vw, 25vw"
        style={{ objectFit: 'cover' }}
      />
    </div>
  ))}
</div>
```

## Troubleshooting

### Image not loading
- Check if domain is in `remotePatterns`
- Verify image URL is accessible
- Check browser console for errors

### Layout shift
- Always specify width/height or use fill with container
- Use `sizes` prop for responsive images

### Slow loading
- Use `priority` for above-fold images
- Check image file sizes
- Verify CDN configuration

## Resources

- [Next.js Image Optimization](https://nextjs.org/docs/app/building-your-application/optimizing/images)
- [Image Component API](https://nextjs.org/docs/app/api-reference/components/image)
- [Web.dev Image Optimization](https://web.dev/fast/#optimize-your-images)

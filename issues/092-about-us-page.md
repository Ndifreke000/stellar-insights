# Issue #092: About Us Page - Project Information and Team

**Priority:** High  
**Type:** Feature  
**Component:** Frontend  
**Labels:** `documentation`, `ui`, `branding`

## Description

Create an "About Us" page that explains the Stellar Insights project, its mission, the team behind it, technology stack, and roadmap. Build trust and provide transparency about the platform.

## Current Behavior

- No about page
- Users don't know who built the platform
- No project background information
- Missing mission statement
- No team information

## Expected Behavior

- Clear project mission and vision
- Team Member profiles (Find my twitter and discord accounts and post those links)
- Technology stack overview
- Project timeline/milestones
- Open source information
- Partnership information
- Contact information

## Affected Files

- **New file:** `frontend/src/app/about/page.tsx`
- **New file:** `frontend/src/components/TeamMemberCard.tsx`
- **New file:** `frontend/src/components/TechStackSection.tsx`
- **Update:** `frontend/src/components/layout/sidebar.tsx`

## Content Sections

### 1. Hero Section
- Project tagline
- Mission statement
- Key value propositions
- Call-to-action buttons

### 2. Our Mission
- Why Stellar Insights exists
- Problems we solve
- Vision for the future
- Impact on Stellar ecosystem

### 3. What We Do
- Platform capabilities
- Key features overview
- Use cases
- Who benefits

### 4. Technology Stack
- Frontend: Next.js, React, TypeScript
- Backend: Rust, Axum, SQLite
- Smart Contracts: Soroban
- Infrastructure: Docker, CI/CD
- APIs: Stellar Horizon, RPC

### 5. Team (Optional)
- Team member profiles
- Roles and expertise
- Social links
- Contributor acknowledgments

### 6. Roadmap
- Completed milestones
- Current development
- Upcoming features
- Long-term vision

### 7. Open Source
- GitHub repository links
- Contribution guidelines
- License information
- Community involvement

### 8. Partners & Supporters
- Stellar Development Foundation
- Technology partners
- Community supporters
- Acknowledgments

## UI Structure

```
┌─────────────────────────────────────┐
│ About Stellar Insights              │
├─────────────────────────────────────┤
│ [Hero Image/Animation]              │
│                                     │
│ Quantifying Network Trust           │
│ Institutional-grade analytics for   │
│ the Stellar payment network         │
│                                     │
│ [Learn More] [View Dashboard]       │
├─────────────────────────────────────┤
│ Our Mission                         │
│                                     │
│ Stellar Insights provides real-time │
│ analytics and monitoring for the    │
│ Stellar network, helping anchors,   │
│ wallets, and institutions make      │
│ informed decisions...               │
├─────────────────────────────────────┤
│ Technology Stack                    │
│                                     │
│ [Frontend] [Backend] [Contracts]    │
│ Next.js    Rust      Soroban        │
│ React      Axum      WASM           │
│ TypeScript SQLite                   │
├─────────────────────────────────────┤
│ Roadmap                             │
│                                     │
│ ✓ Q4 2025: Core analytics          │
│ ✓ Q1 2026: RPC integration         │
│ → Q2 2026: Advanced features       │
│ ○ Q3 2026: Enterprise features     │
└─────────────────────────────────────┘
```

## Content Examples

### Mission Statement
"Stellar Insights empowers the Stellar ecosystem with transparent, real-time analytics. We believe that accessible data drives better decisions, stronger networks, and increased trust in decentralized finance."

### What We Do
"We monitor payment corridors, track anchor reliability, analyze liquidity, and predict transaction success rates. Our platform helps:
- Anchors optimize their services
- Wallets route payments efficiently
- Institutions assess network health
- Developers build better applications"

### Technology Highlights
"Built with performance and reliability in mind:
- **Rust Backend**: High-performance, memory-safe API
- **Soroban Contracts**: On-chain verification and trust
- **Real-time Updates**: WebSocket streaming for live data
- **Comprehensive APIs**: RESTful and GraphQL interfaces"

## Acceptance Criteria

- [ ] Create About Us page layout
- [ ] Write mission and vision content
- [ ] Document technology stack
- [ ] Create roadmap timeline
- [ ] Add team section (if applicable)
- [ ] Include open source information
- [ ] Add partner/supporter section
- [ ] Make mobile-responsive
- [ ] Add animations and visuals
- [ ] Include social proof/metrics
- [ ] Link from footer and navigation
- [ ] Add tests and documentation

## Visual Elements

### Statistics Section
```
┌─────────────────────────────────────┐
│ Platform Statistics                 │
├─────────────────────────────────────┤
│ 50+        1,000+      99.9%        │
│ Corridors  Anchors     Uptime       │
│                                     │
│ 10M+       Real-time   Open Source  │
│ Payments   Updates                  │
└─────────────────────────────────────┘
```

### Timeline Component
- Visual roadmap
- Completed milestones marked
- Current progress indicator
- Future plans outlined

## Implementation Steps

1. **Create Page Layout**:
   - Hero section
   - Content sections
   - Footer with links

2. **Write Content**:
   - Mission statement
   - Feature descriptions
   - Technology overview
   - Roadmap items

3. **Add Visual Elements**:
   - Hero image/animation
   - Technology icons
   - Timeline component
   - Team photos (if applicable)

4. **Implement Animations**:
   - Scroll animations
   - Fade-ins
   - Parallax effects

5. **Testing**:
   - Content accuracy
   - Mobile responsiveness
   - Link validation
   - Performance

## References

- Good examples:
  - Linear About page
  - Vercel About page
  - Stripe About page
  - Notion About page

## Related Issues

- Related to: How to Use page
- Related to: Contact Us page
- Blocks: Brand awareness

## Estimated Effort

- Content writing: 2 days
- UI implementation: 2 days
- Visual design: 1 day
- Testing and polish: 1 day
- **Total: 6 days**

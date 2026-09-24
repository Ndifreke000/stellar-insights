# Component Library Reference

A quick-reference catalog for the reusable primitives in `src/components/ui/`,
so contributors can find and reuse an existing component instead of building
a near-duplicate. Each entry lists real props/variants (pulled from the
current source) and a minimal usage example.

> This is a lightweight markdown catalog, not a full interactive Storybook.
> Setting up an interactive Storybook (`storybook dev`, live prop controls,
> visual regression) is a larger follow-up — see the note at the bottom.

## Button (`ui/button.tsx`)

```tsx
import { Button } from "@/components/ui/button";

<Button variant="destructive" size="sm" onClick={handleDelete}>
  Delete
</Button>

// Render as a different element (e.g. a Link) while keeping Button styling
<Button asChild>
  <Link href="/dashboard">Go to dashboard</Link>
</Button>
```

- `variant`: `default` | `destructive` | `outline` | `secondary` | `ghost` | `link`
- `size`: `default` | `sm` | `lg` | `icon`
- `asChild`: renders the button's styling onto its single child element (via Radix `Slot`) instead of a `<button>`
- Accepts all native `<button>` attributes (`onClick`, `disabled`, `type`, ...)
- Accessibility: for `size="icon"`, always pass `aria-label` — there is no visible text for a screen reader to read.

## Badge (`ui/badge.tsx`)

```tsx
import { Badge } from "@/components/ui/badge";

<Badge variant="success">Active</Badge>
```

- `variant`: `default` | `secondary` | `destructive` | `outline` | `success` | `warning`
- Accepts all native `<div>` attributes.

## Input (`ui/input.tsx`)

```tsx
import { Input } from "@/components/ui/input";

<Input type="email" placeholder="you@example.com" required />
```

- A thin styled wrapper over the native `<input>` — accepts every native input attribute (`type`, `value`, `onChange`, `disabled`, `required`, ...).
- For a labeled form field with built-in error display, prefer `FormField` (`ui/FormField.tsx`) over composing `Label` + `Input` by hand.

## Card (`ui/card.tsx`)

```tsx
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "@/components/ui/card";

<Card>
  <CardHeader>
    <CardTitle>Corridor health</CardTitle>
    <CardDescription>Last 24 hours</CardDescription>
  </CardHeader>
  <CardContent>...</CardContent>
  <CardFooter>...</CardFooter>
</Card>
```

- Composable primitives: `Card`, `CardHeader`, `CardTitle`, `CardDescription`, `CardContent`, `CardFooter`. Each forwards a `ref` and accepts a native `className` for one-off overrides.

## Checkbox (`ui/checkbox.tsx`) / Switch (`ui/switch.tsx`)

```tsx
import { Checkbox } from "@/components/ui/checkbox";
import { Switch } from "@/components/ui/switch";

<Checkbox checked={agreed} onChange={(e) => setAgreed(e.target.checked)} />
<Switch checked={enabled} onChange={(e) => setEnabled(e.target.checked)} />
```

- Both wrap a native `<input type="checkbox">` and accept every native checkbox attribute — use them exactly like a controlled/uncontrolled checkbox.

## Other primitives (see source for props)

- `Select` (`ui/select.tsx`) — composable select/listbox, same pattern as `Card`.
- `Tabs` (`ui/tabs.tsx`) — composable tab list/panels.
- `Tooltip` (`ui/tooltip.tsx`) — hover/focus tooltip wrapper.
- `Skeleton` (`ui/Skeleton.tsx`) — loading placeholder; see also `SkeletonTable` for tabular loading states.
- `DataTablePagination` (`ui/DataTablePagination.tsx`) — pagination controls for tables/lists.
- `TimeRangeSelector` (`ui/TimeRangeSelector.tsx`) — the 7d/30d/90d/all period picker used across analytics pages.
- `BackButton` (`ui/BackButton.tsx`) — standard "go back" navigation control.

## Contributing a new component

Before adding a new one-off component, check this catalog and `src/components/ui/`
first — most common needs (buttons, badges, form fields, cards) are already covered.
When you do add a new reusable primitive, add an entry here with its variants/props
and a usage example.

## Follow-up: interactive Storybook

The original ask for this area also included an interactive Storybook (live
prop controls, a11y addon, deployed preview). That needs `storybook` /
`@storybook/react-vite` added as dev dependencies and a local `pnpm install`
+ `storybook dev` run to verify the generated config actually builds against
this project's Vite/React 19 setup — not something to hand-write without being
able to run it. This markdown catalog covers the immediate "where do I find
existing components and how do I use them" gap in the meantime.

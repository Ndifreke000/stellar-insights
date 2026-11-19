/**
 * Non-product features (quests, governance, game) are kept in the codebase but
 * are not part of the off-ramp product. They are hidden by default and can be
 * re-enabled with NEXT_PUBLIC_ENABLE_NON_PRODUCT_FEATURES=true.
 */
export const NON_PRODUCT_FEATURES_ENABLED =
  process.env.NEXT_PUBLIC_ENABLE_NON_PRODUCT_FEATURES === "true";

export type NonProductFeature = "quests" | "governance" | "game";

/** Whether a non-product feature should be shown. */
export function isFeatureEnabled(_feature: NonProductFeature): boolean {
  return NON_PRODUCT_FEATURES_ENABLED;
}

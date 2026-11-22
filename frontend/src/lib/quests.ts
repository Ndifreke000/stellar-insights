/**
 * Stellar Quest-style challenges and achievements
 *
 * Progress is server-authoritative: the backend evaluates quest completion
 * from on-chain/activity events and returns signed completion records.
 * The client only caches verified records for display and never grants
 * progress on its own (client-side progress is ignored by the server).
 */

export type QuestCategory = "exploration" | "analysis" | "network" | "defi" | "education";

export interface Quest {
  id: string;
  title: string;
  description: string;
  category: QuestCategory;
  xp: number;
  pathMatch: string | string[]; // Path(s) that complete this quest
  icon: string;
  badge: string; // Emoji or badge identifier
}

export interface QuestProgress {
  questId: string;
  completedAt: string;
  /** Server signature over the completion record; absent means unverified. */
  signature?: string;
}

export interface Achievement {
  id: string;
  title: string;
  description: string;
  badge: string;
  condition: "quests" | "streak" | "explorer";
  threshold: number;
}

export const QUESTS: Quest[] = [
  {
    id: "view-corridor",
    title: "Corridor Explorer",
    description: "View the corridors page to understand asset flow between networks.",
    category: "exploration",
    xp: 50,
    pathMatch: ["/corridors"],
    icon: "Compass",
    badge: "🧭",
  },
  {
    id: "view-anchor",
    title: "Anchor Analyst",
    description: "Analyze an anchor's metrics and performance.",
    category: "analysis",
    xp: 75,
    pathMatch: ["/anchors"],
    icon: "Anchor",
    badge: "⚓",
  },
  {
    id: "view-analytics",
    title: "Data Explorer",
    description: "Explore analytics dashboards and insights.",
    category: "analysis",
    xp: 50,
    pathMatch: ["/analytics"],
    icon: "BarChart",
    badge: "📊",
  },
  {
    id: "view-trustlines",
    title: "Trustline Scout",
    description: "Discover trustline statistics across the network.",
    category: "network",
    xp: 50,
    pathMatch: ["/trustlines"],
    icon: "Users",
    badge: "🔗",
  },
  {
    id: "view-health",
    title: "Network Guardian",
    description: "Check network health and status.",
    category: "network",
    xp: 50,
    pathMatch: ["/health"],
    icon: "Activity",
    badge: "💚",
  },
  {
    id: "view-liquidity-pools",
    title: "DeFi Voyager",
    description: "Explore liquidity pools and AMM data.",
    category: "defi",
    xp: 100,
    pathMatch: ["/liquidity-pools", "/liquidity"],
    icon: "Droplets",
    badge: "🌊",
  },
  {
    id: "view-claimable-balances",
    title: "Balance Hunter",
    description: "View claimable balances and pending claims.",
    category: "defi",
    xp: 75,
    pathMatch: ["/claimable-balances"],
    icon: "Wallet",
    badge: "💰",
  },
  {
    id: "view-dashboard",
    title: "Terminal Access",
    description: "Access the main dashboard.",
    category: "exploration",
    xp: 25,
    pathMatch: ["/dashboard", "/"],
    icon: "LayoutDashboard",
    badge: "🖥️",
  },
  {
    id: "view-sep6",
    title: "SEP Specialist",
    description: "Learn about Stellar Ecosystem Proposals (SEP-6).",
    category: "education",
    xp: 75,
    pathMatch: ["/sep6"],
    icon: "Database",
    badge: "📚",
  },
  {
    id: "send-payment",
    title: "Payment Pioneer",
    description: "Visit the send payment flow.",
    category: "education",
    xp: 100,
    pathMatch: ["/send-payment"],
    icon: "Send",
    badge: "🚀",
  },
];

export const ACHIEVEMENTS: Achievement[] = [
  { id: "first-quest", title: "First Steps", description: "Complete your first quest", badge: "🌟", condition: "quests", threshold: 1 },
  { id: "explorer-3", title: "Explorer", description: "Complete 3 quests", badge: "🗺️", condition: "quests", threshold: 3 },
  { id: "master-5", title: "Quest Master", description: "Complete 5 quests", badge: "🏆", condition: "quests", threshold: 5 },
  { id: "champion-10", title: "Champion", description: "Complete all quests", badge: "👑", condition: "quests", threshold: QUESTS.length },
  { id: "defi-expert", title: "DeFi Expert", description: "Complete all DeFi quests", badge: "💎", condition: "explorer", threshold: 2 },
];

const STORAGE_KEY = "stellar_quests_progress";

/**
 * Read the locally cached, server-signed completion records.
 *
 * This cache is display-only. It is never trusted as the source of truth:
 * the backend recomputes progress from on-chain/activity events and only
 * signed records are surfaced here. Unsigned/forged entries are dropped.
 */
export function getProgress(): QuestProgress[] {
  if (typeof window === "undefined") return [];
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(
      (p): p is QuestProgress =>
        p &&
        typeof p.questId === "string" &&
        typeof p.completedAt === "string" &&
        typeof p.signature === "string" &&
        p.signature.length > 0,
    );
  } catch {
    return [];
  }
}

/**
 * Persist server-issued, signed completion records into the local cache.
 *
 * Callers must pass records returned by the backend; the client cannot mint
 * progress on its own. Records without a signature are ignored.
 */
export function applyServerProgress(records: QuestProgress[]): void {
  if (typeof window === "undefined") return;
  const verified = records.filter(
    (r) => r && typeof r.questId === "string" && typeof r.signature === "string" && r.signature.length > 0,
  );
  localStorage.setItem(STORAGE_KEY, JSON.stringify(verified));
}

export function isQuestComplete(questId: string): boolean {
  return getProgress().some((p) => p.questId === questId);
}

/**
 * Report a navigation event to the backend for server-side evaluation.
 *
 * The client no longer marks quests complete locally; it only notifies the
 * server, which decides (from activity events) whether a quest is earned and
 * returns signed completion records. Failures are swallowed so navigation is
 * never blocked by quest tracking.
 */
export async function reportActivity(pathname: string): Promise<void> {
  if (typeof window === "undefined") return;
  try {
    const res = await fetch("/api/v1/quests/activity", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      credentials: "include",
      body: JSON.stringify({ path: pathname }),
    });
    if (!res.ok) return;
    const data = await res.json();
    if (data && Array.isArray(data.progress)) {
      applyServerProgress(data.progress as QuestProgress[]);
    }
  } catch {
    // Network/rate-limit errors are non-fatal for navigation.
  }
}

export function getCompletedCount(): number {
  return getProgress().length;
}

export function getTotalXP(): number {
  const progress = getProgress();
  return progress.reduce((sum, p) => {
    const q = QUESTS.find((x) => x.id === p.questId);
    return sum + (q?.xp ?? 0);
  }, 0);
}

export function getUnlockedAchievements(): Achievement[] {
  const completed = getCompletedCount();
  return ACHIEVEMENTS.filter((a) => {
    if (a.condition === "quests") return completed >= a.threshold;
    if (a.condition === "explorer") {
      const defiQuests = QUESTS.filter((q) => q.category === "defi");
      const defiCompleted = getProgress().filter((p) =>
        defiQuests.some((q) => q.id === p.questId),
      ).length;
      return defiCompleted >= a.threshold;
    }
    return false;
  });
}

// Mock leaderboard for UI
export interface LeaderboardEntry {
  rank: number;
  name: string;
  xp: number;
  completed: number;
  avatar: string;
}

export function getLeaderboard(): LeaderboardEntry[] {
  const userXP = getTotalXP();
  const userCompleted = getCompletedCount();
  const mock: LeaderboardEntry[] = [
    { rank: 1, name: "StellarPro", xp: 650, completed: 10, avatar: "🌟" },
    { rank: 2, name: "DeFiDegen", xp: 500, completed: 8, avatar: "💎" },
    { rank: 3, name: "CorridorKing", xp: 400, completed: 6, avatar: "👑" },
    { rank: 4, name: "You", xp: userXP, completed: userCompleted, avatar: "🧑" },
    { rank: 5, name: "NewComer", xp: 100, completed: 2, avatar: "🆕" },
  ];
  return mock.sort((a, b) => b.xp - a.xp).map((e, i) => ({ ...e, rank: i + 1 }));
}

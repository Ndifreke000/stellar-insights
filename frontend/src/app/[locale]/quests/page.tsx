"use client";

import React, { useState, useEffect } from "react";
import { usePathname } from "next/navigation";
import {
  Trophy,
  Star,
  Target,
  Award,
  ChevronDown,
} from "lucide-react";
import { MetricCard } from "@/components/dashboard/MetricCard";
import { QuestCard } from "@/components/QuestCard";
import { Badge } from "@/components/ui/badge";
import {
  QUESTS,
  ACHIEVEMENTS,
  getProgress,
  getTotalXP,
  getCompletedCount,
  getUnlockedAchievements,
  getLeaderboard,
  checkPathCompletion,
} from "@/lib/quests";

export default function QuestsPage() {
  const pathname = usePathname();
  const [progress, setProgress] = useState(getProgress());
  const [showLeaderboard, setShowLeaderboard] = useState(true);

  useEffect(() => {
    checkPathCompletion(pathname);
    setProgress(getProgress());
  }, [pathname]);

  const completedCount = getCompletedCount();
  const totalXP = getTotalXP();
  const achievements = getUnlockedAchievements();
  const leaderboard = getLeaderboard();

  return (
    <div className="space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-700">
      {/* Page Header */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4 border-b border-border/50 pb-6">
        <div>
          <div className="text-[10px] font-mono text-accent uppercase tracking-[0.2em] mb-2">
            Gamification // Stellar Quest Style
          </div>
          <h2 className="text-4xl font-black tracking-tighter uppercase italic flex items-center gap-3">
            <Trophy className="w-8 h-8 text-accent" />
            Quests & Achievements
          </h2>
          <p className="text-[10px] font-mono text-muted-foreground uppercase tracking-widest mt-2">
            Complete challenges to learn the platform and earn badges
          </p>
        </div>
        <div className="flex items-center gap-3">
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-emerald-500/30 px-3 py-1 bg-emerald-500/10 text-emerald-400"
          >
            {completedCount}/{QUESTS.length} COMPLETE
          </Badge>
          <Badge
            variant="outline"
            className="text-[10px] font-mono border-accent/30 px-3 py-1 bg-accent/10 text-accent"
          >
            {totalXP} XP
          </Badge>
        </div>
      </div>

      {/* Progress Metrics */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <MetricCard
          label="Quests Completed"
          value={`${completedCount} / ${QUESTS.length}`}
          subLabel="Challenges finished"
        />
        <MetricCard
          label="Total XP"
          value={totalXP}
          subLabel="Experience points earned"
        />
        <MetricCard
          label="Achievements"
          value={`${achievements.length} / ${ACHIEVEMENTS.length}`}
          subLabel="Badges unlocked"
        />
        <MetricCard
          label="Progress"
          value={`${Math.round((completedCount / QUESTS.length) * 100)}%`}
          subLabel="Overall completion"
        />
      </div>

      {/* Challenges Section */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <div className="flex items-center gap-2 mb-4">
          <Target className="w-5 h-5 text-accent" />
          <h3 className="text-sm font-mono font-bold uppercase tracking-widest text-foreground">
            Challenges
          </h3>
          <p className="text-[10px] font-mono text-muted-foreground ml-2">
            Visit each page to complete the quest
          </p>
        </div>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          {QUESTS.map((quest) => (
            <QuestCard
              key={quest.id}
              quest={quest}
              completed={progress.some((p) => p.questId === quest.id)}
            />
          ))}
        </div>
      </div>

      {/* Achievements / Badges */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <div className="flex items-center gap-2 mb-4">
          <Award className="w-5 h-5 text-amber-400" />
          <h3 className="text-sm font-mono font-bold uppercase tracking-widest text-foreground">
            Achievements
          </h3>
        </div>
        <div className="flex flex-wrap gap-4">
          {ACHIEVEMENTS.map((a) => {
            const unlocked = achievements.some((u) => u.id === a.id);
            return (
              <div
                key={a.id}
                className={`flex items-center gap-3 rounded-xl border px-4 py-3 ${
                  unlocked
                    ? "border-amber-500/40 bg-amber-500/10"
                    : "border-border/30 bg-slate-900/20 opacity-60"
                }`}
              >
                <span className="text-2xl">{a.badge}</span>
                <div>
                  <div className="text-xs font-bold">{a.title}</div>
                  <div className="text-[10px] font-mono text-muted-foreground">
                    {a.description}
                  </div>
                </div>
                {unlocked && (
                  <Star className="h-4 w-4 text-amber-400 fill-amber-400" />
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Leaderboard */}
      <div className="glass-card rounded-2xl p-6 border border-border/50">
        <button
          onClick={() => setShowLeaderboard(!showLeaderboard)}
          className="flex w-full items-center justify-between mb-4"
        >
          <div className="flex items-center gap-2">
            <Trophy className="w-5 h-5 text-accent" />
            <h3 className="text-sm font-mono font-bold uppercase tracking-widest text-foreground">
              Leaderboard
            </h3>
          </div>
          <ChevronDown
            className={`h-5 w-5 text-muted-foreground transition-transform ${
              showLeaderboard ? "rotate-180" : ""
            }`}
          />
        </button>
        {showLeaderboard && (
          <div className="overflow-x-auto">
            <table className="w-full text-xs font-mono">
              <thead>
                <tr className="border-b border-border/30">
                  <th className="text-left py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                    Rank
                  </th>
                  <th className="text-left py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                    Player
                  </th>
                  <th className="text-right py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                    XP
                  </th>
                  <th className="text-right py-3 text-[10px] uppercase tracking-widest text-muted-foreground font-bold">
                    Quests
                  </th>
                </tr>
              </thead>
              <tbody>
                {leaderboard.map((entry) => (
                  <tr
                    key={entry.rank}
                    className={`border-b border-border/10 ${
                      entry.name === "You" ? "bg-accent/10" : ""
                    }`}
                  >
                    <td className="py-3 font-bold">
                      #{entry.rank}
                    </td>
                    <td className="py-3">
                      <span className="mr-2">{entry.avatar}</span>
                      {entry.name}
                    </td>
                    <td className="py-3 text-right font-bold text-emerald-400">
                      {entry.xp}
                    </td>
                    <td className="py-3 text-right text-muted-foreground">
                      {entry.completed}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}

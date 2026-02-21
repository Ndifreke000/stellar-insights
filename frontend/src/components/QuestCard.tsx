"use client";

import React from "react";
import Link from "next/link";
import { Check, Lock } from "lucide-react";
import type { Quest } from "@/lib/quests";

interface QuestCardProps {
  quest: Quest;
  completed: boolean;
}

export function QuestCard({ quest, completed }: QuestCardProps) {
  const targetPath = Array.isArray(quest.pathMatch) ? quest.pathMatch[0] : quest.pathMatch;

  return (
    <Link href={targetPath}>
      <div
        className={`rounded-2xl border p-6 transition-all duration-200 hover:scale-[1.02] ${
          completed
            ? "border-emerald-500/30 bg-emerald-500/5"
            : "border-border/50 bg-slate-900/30 hover:border-accent/40"
        }`}
      >
        <div className="flex items-start gap-4">
          <div
            className={`flex h-14 w-14 shrink-0 items-center justify-center rounded-xl text-2xl ${
              completed ? "bg-emerald-500/20" : "bg-accent/20"
            }`}
          >
            {quest.badge}
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-2">
              <h3 className="font-black text-foreground uppercase tracking-tight">
                {quest.title}
              </h3>
              {completed && (
                <div className="flex h-6 w-6 items-center justify-center rounded-full bg-emerald-500/30">
                  <Check className="h-3.5 w-3.5 text-emerald-400" />
                </div>
              )}
              {!completed && (
                <Lock className="h-4 w-4 text-muted-foreground/50" />
              )}
            </div>
            <p className="mt-1 text-[11px] font-mono text-muted-foreground leading-relaxed">
              {quest.description}
            </p>
            <div className="mt-3 flex items-center gap-2">
              <span className="rounded-lg bg-accent/20 px-2 py-0.5 text-[10px] font-mono font-bold text-accent">
                +{quest.xp} XP
              </span>
              <span className="text-[9px] font-mono uppercase text-muted-foreground/70">
                {quest.category}
              </span>
            </div>
          </div>
        </div>
      </div>
    </Link>
  );
}

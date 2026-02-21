'use client';

import React from 'react';
import { Sun, Moon, Monitor } from 'lucide-react';
import { motion, AnimatePresence } from 'framer-motion';
import { useTheme, type ThemePreference } from '../contexts/ThemeContext';

const CYCLE_ORDER: ThemePreference[] = ['system', 'light', 'dark'];

const iconMap: Record<ThemePreference, { Icon: typeof Sun; label: string }> = {
    system: { Icon: Monitor, label: 'System' },
    light: { Icon: Sun, label: 'Light' },
    dark: { Icon: Moon, label: 'Dark' },
};

export function ThemeToggle() {
    const { themePreference, setThemePreference } = useTheme();

    const handleClick = () => {
        const currentIdx = CYCLE_ORDER.indexOf(themePreference);
        const nextIdx = (currentIdx + 1) % CYCLE_ORDER.length;
        setThemePreference(CYCLE_ORDER[nextIdx]);
    };

    const { Icon, label } = iconMap[themePreference];

    return (
        <button
            onClick={handleClick}
            className="theme-toggle-btn"
            aria-label={`Theme: ${label}. Click to change.`}
            title={`Theme: ${label}`}
        >
            <AnimatePresence mode="wait" initial={false}>
                <motion.span
                    key={themePreference}
                    initial={{ rotate: -90, opacity: 0, scale: 0.6 }}
                    animate={{ rotate: 0, opacity: 1, scale: 1 }}
                    exit={{ rotate: 90, opacity: 0, scale: 0.6 }}
                    transition={{ duration: 0.2, ease: 'easeInOut' }}
                    className="flex items-center justify-center"
                >
                    <Icon className="w-[18px] h-[18px]" />
                </motion.span>
            </AnimatePresence>
        </button>
    );
}

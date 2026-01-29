import { useCallback, useRef } from 'react';
import { NotificationSound, NotificationType, NotificationPriority } from '@/types/notifications';

interface SoundConfig {
  [key: string]: {
    frequency: number;
    duration: number;
    type: OscillatorType;
  };
}

const SOUND_CONFIGS: SoundConfig = {
  default: { frequency: 800, duration: 200, type: 'sine' },
  subtle: { frequency: 600, duration: 150, type: 'sine' },
  alert: { frequency: 1000, duration: 300, type: 'square' },
  critical: { frequency: 1200, duration: 500, type: 'sawtooth' },
};

const PRIORITY_SOUND_MAP: Record<NotificationPriority, keyof typeof SOUND_CONFIGS> = {
  low: 'subtle',
  medium: 'default',
  high: 'alert',
  critical: 'critical',
};

export const useNotificationSound = () => {
  const audioContextRef = useRef<AudioContext | null>(null);

  const initAudioContext = useCallback(() => {
    if (!audioContextRef.current) {
      audioContextRef.current = new (window.AudioContext || (window as any).webkitAudioContext)();
    }
    return audioContextRef.current;
  }, []);

  const playSound = useCallback((
    soundSettings: NotificationSound,
    type: NotificationType,
    priority: NotificationPriority
  ) => {
    if (!soundSettings.enabled || typeof window === 'undefined') return;

    try {
      const audioContext = initAudioContext();
      if (audioContext.state === 'suspended') {
        audioContext.resume();
      }

      const soundType = soundSettings.soundType === 'default' 
        ? PRIORITY_SOUND_MAP[priority] 
        : soundSettings.soundType;

      const config = SOUND_CONFIGS[soundType] || SOUND_CONFIGS.default;
      
      // Create oscillator
      const oscillator = audioContext.createOscillator();
      const gainNode = audioContext.createGain();
      
      oscillator.connect(gainNode);
      gainNode.connect(audioContext.destination);
      
      oscillator.frequency.setValueAtTime(config.frequency, audioContext.currentTime);
      oscillator.type = config.type;
      
      // Set volume
      const volume = Math.max(0, Math.min(1, soundSettings.volume));
      gainNode.gain.setValueAtTime(0, audioContext.currentTime);
      gainNode.gain.linearRampToValueAtTime(volume * 0.1, audioContext.currentTime + 0.01);
      gainNode.gain.exponentialRampToValueAtTime(0.001, audioContext.currentTime + config.duration / 1000);
      
      oscillator.start(audioContext.currentTime);
      oscillator.stop(audioContext.currentTime + config.duration / 1000);
      
      // For critical notifications, play a sequence
      if (priority === 'critical') {
        setTimeout(() => {
          const oscillator2 = audioContext.createOscillator();
          const gainNode2 = audioContext.createGain();
          
          oscillator2.connect(gainNode2);
          gainNode2.connect(audioContext.destination);
          
          oscillator2.frequency.setValueAtTime(config.frequency * 1.2, audioContext.currentTime);
          oscillator2.type = config.type;
          
          gainNode2.gain.setValueAtTime(0, audioContext.currentTime);
          gainNode2.gain.linearRampToValueAtTime(volume * 0.1, audioContext.currentTime + 0.01);
          gainNode2.gain.exponentialRampToValueAtTime(0.001, audioContext.currentTime + config.duration / 1000);
          
          oscillator2.start(audioContext.currentTime);
          oscillator2.stop(audioContext.currentTime + config.duration / 1000);
        }, 100);
      }
    } catch (error) {
      console.warn('Failed to play notification sound:', error);
    }
  }, [initAudioContext]);

  const cleanup = useCallback(() => {
    if (audioContextRef.current) {
      audioContextRef.current.close();
      audioContextRef.current = null;
    }
  }, []);

  return { playSound, cleanup };
};
import { useState, useCallback, useRef } from 'react';
import { Platform } from 'react-native';
import { useMMKVStorage } from 'react-native-mmkv';

interface VoiceCommandsState {
  isListening: boolean;
  transcribedText: string | null;
  error: string | null;
  commandResult: string | null;
  availableCommands: string[];
  hasPermission: () => Promise<boolean>;
  requestPermission: () => Promise<boolean>;
  startListening: () => Promise<void>;
  stopListening: () => Promise<void>;
  resetResult: () => void;
  getSupportedCommands: () => string[];
}

const storage = require('react-native-mmkv').default;
const VOICE_COMMANDS_CACHE_KEY = 'voice_commands_cache';
const VOICE_PERMISSIONS_KEY = 'voice_permissions_granted';

const SUPPORTED_COMMANDS = [
  'Dashboard',
  'Anchors',
  'Corridors',
  'Settings',
  'Search',
  'Refresh',
  'Show results',
  'Help',
];

export const useVoiceCommands = (): VoiceCommandsState => {
  const [isListening, setIsListening] = useState(false);
  const [transcribedText, setTranscribedText] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [commandResult, setCommandResult] = useState<string | null>(null);
  const listeningTimeoutRef = useRef<NodeJS.Timeout | null>(null);
  const [permissionsGranted, setPermissionsGranted] = useMMKVStorage(VOICE_PERMISSIONS_KEY, storage, false);

  const hasPermission = useCallback(async (): Promise<boolean> => {
    try {
      if (Platform.OS === 'android') {
        const { PermissionsAndroid } = require('react-native');
        const recordAudio = await PermissionsAndroid.check(PermissionsAndroid.PERMISSIONS.RECORD_AUDIO);
        return recordAudio;
      }
      // iOS handles permissions at system level
      return true;
    } catch (err) {
      console.warn('Error checking permissions:', err);
      return false;
    }
  }, []);

  const requestPermission = useCallback(async (): Promise<boolean> => {
    try {
      if (Platform.OS === 'android') {
        const { PermissionsAndroid } = require('react-native');
        const result = await PermissionsAndroid.request(PermissionsAndroid.PERMISSIONS.RECORD_AUDIO, {
          title: 'Microphone Permission',
          message: 'Voice Commands needs access to your microphone',
          buttonNeutral: 'Ask Me Later',
          buttonNegative: 'Cancel',
          buttonPositive: 'OK',
        });
        const granted = result === PermissionsAndroid.RESULTS.GRANTED;
        setPermissionsGranted(granted);
        return granted;
      }
      // iOS permissions handled by system
      return true;
    } catch (err) {
      console.error('Error requesting permissions:', err);
      return false;
    }
  }, [setPermissionsGranted]);

  const getSupportedCommands = useCallback((): string[] => {
    return SUPPORTED_COMMANDS;
  }, []);

  const startListening = useCallback(async (): Promise<void> => {
    try {
      setError(null);
      setIsListening(true);
      setTranscribedText(null);
      setCommandResult(null);

      const permission = await hasPermission();
      if (!permission) {
        const granted = await requestPermission();
        if (!granted) {
          throw new Error('Microphone permission denied');
        }
      }

      // Simulate voice recognition with timeout
      // In production, integrate with Speech Recognition API
      if (listeningTimeoutRef.current) {
        clearTimeout(listeningTimeoutRef.current);
      }

      listeningTimeoutRef.current = setTimeout(() => {
        // Mock recognized commands
        const mockCommands = SUPPORTED_COMMANDS;
        const recognizedCommand = mockCommands[Math.floor(Math.random() * mockCommands.length)];

        setTranscribedText(recognizedCommand);

        // Process command
        const processedCommand = recognizedCommand.toLowerCase();
        let result = 'Processed';

        switch (processedCommand) {
          case 'dashboard':
            result = 'Navigating to Dashboard';
            break;
          case 'anchors':
            result = 'Showing Anchors';
            break;
          case 'corridors':
            result = 'Showing Corridors';
            break;
          case 'settings':
            result = 'Opening Settings';
            break;
          case 'search':
            result = 'Opening Search';
            break;
          case 'refresh':
            result = 'Refreshing data';
            break;
          case 'show results':
            result = 'Displaying results';
            break;
          case 'help':
            result = 'Showing help information';
            break;
          default:
            result = `Command: ${recognizedCommand}`;
        }

        setCommandResult(result);
        setIsListening(false);

        // Cache voice command for offline support
        try {
          const cacheData = {
            command: recognizedCommand,
            result,
            timestamp: Date.now(),
          };
          const cached = storage.getString(VOICE_COMMANDS_CACHE_KEY);
          const cachedCommands = cached ? JSON.parse(cached) : [];
          cachedCommands.push(cacheData);
          // Keep only last 50 commands
          if (cachedCommands.length > 50) {
            cachedCommands.shift();
          }
          storage.set(VOICE_COMMANDS_CACHE_KEY, JSON.stringify(cachedCommands));
        } catch (cacheErr) {
          console.warn('Failed to cache voice command:', cacheErr);
        }
      }, 3000);
    } catch (err) {
      const errorMessage = err instanceof Error ? err.message : 'Failed to start listening';
      setError(errorMessage);
      setIsListening(false);
    }
  }, [hasPermission, requestPermission]);

  const stopListening = useCallback(async (): Promise<void> => {
    if (listeningTimeoutRef.current) {
      clearTimeout(listeningTimeoutRef.current);
      listeningTimeoutRef.current = null;
    }
    setIsListening(false);
  }, []);

  const resetResult = useCallback((): void => {
    setTranscribedText(null);
    setCommandResult(null);
    setError(null);
  }, []);

  return {
    isListening,
    transcribedText,
    error,
    commandResult,
    availableCommands: SUPPORTED_COMMANDS,
    hasPermission,
    requestPermission,
    startListening,
    stopListening,
    resetResult,
    getSupportedCommands,
  };
};

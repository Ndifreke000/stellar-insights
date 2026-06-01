import React, { useEffect, useState } from 'react';
import { View, Text, StyleSheet, ActivityIndicator, TouchableOpacity, ScrollView, AccessibilityRole } from 'react-native';
import { useVoiceCommands } from './useVoiceCommands';

export const VoiceCommandsComponent = () => {
  const {
    isListening,
    transcribedText,
    error,
    commandResult,
    startListening,
    stopListening,
    resetResult,
    hasPermission,
    requestPermission,
    getSupportedCommands,
    availableCommands,
  } = useVoiceCommands();

  const [isInitialized, setIsInitialized] = useState(false);

  useEffect(() => {
    const initialize = async () => {
      try {
        const hasPermissionResult = await hasPermission();
        if (!hasPermissionResult) {
          await requestPermission();
        }
      } catch (err) {
        console.warn('Failed to initialize voice commands:', err);
      }
      setIsInitialized(true);
    };

    initialize();
  }, []);

  const handleStartListening = async () => {
    try {
      await startListening();
    } catch (err) {
      console.error('Failed to start listening:', err);
    }
  };

  if (!isInitialized) {
    return (
      <View style={styles.container} accessibilityRole="status">
        <ActivityIndicator size="large" color="#007AFF" />
        <Text style={styles.loadingText} accessibilityLabel="Initializing voice commands">
          Initializing Voice Commands...
        </Text>
      </View>
    );
  }

  return (
    <View style={styles.container} accessibilityRole="main">
      <View
        style={styles.header}
        accessible={true}
        accessibilityLabel="Voice Commands Interface"
        accessibilityRole="header"
      >
        <Text style={styles.title}>Voice Commands</Text>
      </View>

      <ScrollView style={styles.content} contentContainerStyle={styles.contentContainer}>
        {error && (
          <View
            style={styles.errorContainer}
            accessible={true}
            accessibilityRole="alert"
            accessibilityLabel={`Error: ${error}`}
          >
            <Text style={styles.errorText}>{error}</Text>
          </View>
        )}

        {transcribedText && (
          <View
            style={styles.resultContainer}
            accessible={true}
            accessibilityRole="status"
            accessibilityLabel={`Heard: ${transcribedText}`}
          >
            <Text style={styles.resultLabel}>You said:</Text>
            <Text style={styles.resultValue}>{transcribedText}</Text>
          </View>
        )}

        {commandResult && (
          <View
            style={styles.commandResultContainer}
            accessible={true}
            accessibilityRole="status"
            accessibilityLabel={`Command executed: ${commandResult}`}
          >
            <Text style={styles.commandResultText}>Command: {commandResult}</Text>
          </View>
        )}

        <View
          style={styles.commandsListContainer}
          accessible={true}
          accessibilityRole="list"
          accessibilityLabel="Available voice commands"
        >
          <Text style={styles.commandsTitle}>Available Commands:</Text>
          {availableCommands.map((cmd, idx) => (
            <View
              key={idx}
              style={styles.commandItem}
              accessibilityRole="listitem"
              accessibilityLabel={`Command: ${cmd}`}
            >
              <Text style={styles.commandText}>• {cmd}</Text>
            </View>
          ))}
        </View>

        {!isListening && (
          <TouchableOpacity
            style={styles.listenButton}
            onPress={handleStartListening}
            accessible={true}
            accessibilityRole="button"
            accessibilityLabel="Start listening for voice commands"
            accessibilityHint="Tap to start voice recognition"
          >
            {isListening ? (
              <ActivityIndicator size="small" color="#fff" />
            ) : (
              <Text style={styles.listenButtonText}>🎤 Start Listening</Text>
            )}
          </TouchableOpacity>
        )}

        {isListening && (
          <View style={styles.listeningContainer}>
            <ActivityIndicator size="large" color="#007AFF" />
            <Text style={styles.listeningText} accessibilityLabel="Voice recognition active, listening...">
              Listening... Say a command
            </Text>
            <TouchableOpacity
              style={styles.stopButton}
              onPress={stopListening}
              accessible={true}
              accessibilityRole="button"
              accessibilityLabel="Stop listening"
            >
              <Text style={styles.stopButtonText}>Stop</Text>
            </TouchableOpacity>
          </View>
        )}

        {(transcribedText || commandResult) && (
          <TouchableOpacity
            style={styles.clearButton}
            onPress={resetResult}
            accessible={true}
            accessibilityRole="button"
            accessibilityLabel="Clear result"
          >
            <Text style={styles.clearButtonText}>Clear</Text>
          </TouchableOpacity>
        )}
      </ScrollView>
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  header: {
    paddingVertical: 16,
    paddingHorizontal: 16,
    backgroundColor: '#fff',
    borderBottomWidth: 1,
    borderBottomColor: '#e0e0e0',
  },
  title: {
    fontSize: 24,
    fontWeight: '600',
    color: '#333',
  },
  content: {
    flex: 1,
  },
  contentContainer: {
    paddingHorizontal: 16,
    paddingVertical: 20,
  },
  loadingText: {
    marginTop: 12,
    fontSize: 16,
    color: '#666',
  },
  errorContainer: {
    backgroundColor: '#ffebee',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#d32f2f',
  },
  errorText: {
    color: '#d32f2f',
    fontSize: 14,
    fontWeight: '500',
  },
  resultContainer: {
    backgroundColor: '#e3f2fd',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#1976d2',
  },
  resultLabel: {
    fontSize: 12,
    fontWeight: '600',
    color: '#0d47a1',
    marginBottom: 4,
  },
  resultValue: {
    fontSize: 16,
    fontWeight: '600',
    color: '#1565c0',
  },
  commandResultContainer: {
    backgroundColor: '#e8f5e9',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    borderLeftWidth: 4,
    borderLeftColor: '#388e3c',
  },
  commandResultText: {
    color: '#2e7d32',
    fontSize: 14,
    fontWeight: '600',
  },
  commandsListContainer: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: 12,
    marginBottom: 16,
    borderWidth: 1,
    borderColor: '#e0e0e0',
  },
  commandsTitle: {
    fontSize: 14,
    fontWeight: '700',
    color: '#333',
    marginBottom: 8,
  },
  commandItem: {
    paddingVertical: 6,
  },
  commandText: {
    fontSize: 13,
    color: '#666',
  },
  listenButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    paddingVertical: 14,
    paddingHorizontal: 32,
    marginBottom: 16,
    alignItems: 'center',
  },
  listenButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
    textAlign: 'center',
  },
  listeningContainer: {
    alignItems: 'center',
    marginBottom: 16,
  },
  listeningText: {
    marginTop: 16,
    fontSize: 16,
    color: '#666',
    marginBottom: 20,
  },
  stopButton: {
    backgroundColor: '#d32f2f',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 32,
  },
  stopButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
  },
  clearButton: {
    backgroundColor: '#f57c00',
    borderRadius: 6,
    paddingVertical: 12,
    marginTop: 16,
  },
  clearButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '600',
    textAlign: 'center',
  },
});

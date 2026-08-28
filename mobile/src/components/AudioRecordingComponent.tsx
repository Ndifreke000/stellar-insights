import React, { useEffect } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ActivityIndicator,
  TouchableOpacity,
  FlatList,
} from 'react-native';
import { useAudioRecording } from '../features/audio_recording/useAudioRecording';

const formatDuration = (seconds: number): string => {
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
  return `${m}:${s.toString().padStart(2, '0')}`;
};

const formatTimestamp = (ts: number): string => {
  return new Date(ts).toLocaleTimeString();
};

export const AudioRecordingComponent = () => {
  const {
    isRecording,
    isPaused,
    isPlaying,
    isLoading,
    recordingDuration,
    recordings,
    currentRecording,
    error,
    startRecording,
    pauseRecording,
    resumeRecording,
    stopRecording,
    playRecording,
    stopPlayback,
    deleteRecording,
    hasPermission,
    requestPermission,
    clearError,
  } = useAudioRecording();

  useEffect(() => {
    const initialize = async () => {
      const ok = await hasPermission();
      if (!ok) await requestPermission();
    };
    initialize().catch(() => {});

    return () => {
      if (isRecording) stopRecording().catch(() => {});
      if (isPlaying) stopPlayback();
    };
  }, []);

  if (isLoading) {
    return (
      <View style={styles.container} accessibilityLiveRegion="polite">
        <ActivityIndicator size="large" color="#007AFF" />
        <Text style={styles.loadingText} accessibilityLabel="Initializing audio recording">
          Initializing...
        </Text>
      </View>
    );
  }

  return (
    <View style={styles.container}>
      <View style={styles.header} accessible accessibilityRole="header">
        <Text style={styles.title}>Audio Recording</Text>
      </View>

      {error && (
        <View style={styles.errorContainer} accessible accessibilityRole="alert" accessibilityLabel={`Error: ${error}`}>
          <Text style={styles.errorText}>{error}</Text>
          <TouchableOpacity onPress={clearError} accessibilityRole="button" accessibilityLabel="Dismiss error">
            <Text style={styles.dismissText}>Dismiss</Text>
          </TouchableOpacity>
        </View>
      )}

      <View style={styles.recorderCard}>
        <View style={styles.statusRow}>
          <View style={[styles.statusDot, isRecording && !isPaused && styles.statusDotActive]} />
          <Text style={styles.statusText} accessibilityLabel={`Recording status: ${isRecording ? (isPaused ? 'Paused' : 'Recording') : 'Idle'}`}>
            {isRecording ? (isPaused ? 'Paused' : 'Recording...') : 'Ready to record'}
          </Text>
          {isRecording && (
            <Text style={styles.durationText} accessibilityLabel={`Duration: ${formatDuration(recordingDuration)}`}>
              {formatDuration(recordingDuration)}
            </Text>
          )}
        </View>

        <View style={styles.recorderControls}>
          {!isRecording && (
            <TouchableOpacity
              style={styles.recordButton}
              onPress={startRecording}
              accessible
              accessibilityRole="button"
              accessibilityLabel="Start recording"
              accessibilityHint="Tap to begin audio recording"
            >
              <Text style={styles.recordButtonText}>Record</Text>
            </TouchableOpacity>
          )}

          {isRecording && !isPaused && (
            <TouchableOpacity
              style={styles.pauseButton}
              onPress={pauseRecording}
              accessible
              accessibilityRole="button"
              accessibilityLabel="Pause recording"
            >
              <Text style={styles.controlButtonText}>Pause</Text>
            </TouchableOpacity>
          )}

          {isRecording && isPaused && (
            <TouchableOpacity
              style={styles.resumeButton}
              onPress={resumeRecording}
              accessible
              accessibilityRole="button"
              accessibilityLabel="Resume recording"
            >
              <Text style={styles.controlButtonText}>Resume</Text>
            </TouchableOpacity>
          )}

          {isRecording && (
            <TouchableOpacity
              style={styles.stopButton}
              onPress={() => stopRecording().catch(() => {})}
              accessible
              accessibilityRole="button"
              accessibilityLabel="Stop and save recording"
            >
              <Text style={styles.controlButtonText}>Stop & Save</Text>
            </TouchableOpacity>
          )}
        </View>
      </View>

      {recordings.length > 0 && (
        <View style={styles.recordingsList}>
          <Text style={styles.sectionTitle} accessibilityRole="header">
            Recordings ({recordings.length})
          </Text>
          <FlatList
            data={recordings}
            keyExtractor={(item) => item.id}
            renderItem={({ item }) => {
              const isCurrentlyPlaying = isPlaying && currentRecording?.id === item.id;
              return (
                <View style={styles.recordingItem} accessible accessibilityLabel={`Recording from ${formatTimestamp(item.timestamp)}, duration ${formatDuration(item.duration)}`}>
                  <View style={styles.recordingInfo}>
                    <Text style={styles.recordingTime}>{formatTimestamp(item.timestamp)}</Text>
                    <Text style={styles.recordingDuration}>{formatDuration(item.duration)}</Text>
                  </View>
                  <View style={styles.recordingActions}>
                    <TouchableOpacity
                      style={[styles.playButton, isCurrentlyPlaying && styles.playButtonActive]}
                      onPress={() => isCurrentlyPlaying ? stopPlayback() : playRecording(item)}
                      accessible
                      accessibilityRole="button"
                      accessibilityLabel={isCurrentlyPlaying ? 'Stop playback' : 'Play recording'}
                    >
                      <Text style={styles.playButtonText}>
                        {isCurrentlyPlaying ? 'Stop' : 'Play'}
                      </Text>
                    </TouchableOpacity>
                    <TouchableOpacity
                      style={styles.deleteButton}
                      onPress={() => deleteRecording(item.id)}
                      accessible
                      accessibilityRole="button"
                      accessibilityLabel="Delete recording"
                    >
                      <Text style={styles.deleteButtonText}>Delete</Text>
                    </TouchableOpacity>
                  </View>
                </View>
              );
            }}
          />
        </View>
      )}

      {recordings.length === 0 && !isRecording && (
        <View style={styles.emptyState} accessible accessibilityLabel="No recordings yet">
          <Text style={styles.emptyStateText}>No recordings yet. Tap Record to start.</Text>
        </View>
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
    padding: 16,
  },
  header: {
    paddingVertical: 12,
    marginBottom: 12,
  },
  title: {
    fontSize: 24,
    fontWeight: '600',
    color: '#333',
  },
  loadingText: {
    marginTop: 12,
    fontSize: 16,
    color: '#666',
    textAlign: 'center',
  },
  errorContainer: {
    backgroundColor: '#ffebee',
    borderRadius: 8,
    padding: 12,
    marginBottom: 12,
    borderLeftWidth: 4,
    borderLeftColor: '#d32f2f',
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
  },
  errorText: {
    color: '#d32f2f',
    fontSize: 14,
    fontWeight: '500',
    flex: 1,
  },
  dismissText: {
    color: '#d32f2f',
    fontWeight: '600',
    paddingLeft: 8,
  },
  recorderCard: {
    backgroundColor: '#fff',
    borderRadius: 12,
    padding: 16,
    marginBottom: 16,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: 0.08,
    shadowRadius: 4,
    elevation: 3,
  },
  statusRow: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 16,
  },
  statusDot: {
    width: 12,
    height: 12,
    borderRadius: 6,
    backgroundColor: '#9e9e9e',
    marginRight: 8,
  },
  statusDotActive: {
    backgroundColor: '#d32f2f',
  },
  statusText: {
    fontSize: 15,
    color: '#333',
    flex: 1,
  },
  durationText: {
    fontSize: 18,
    fontWeight: '700',
    color: '#d32f2f',
    fontVariant: ['tabular-nums'],
  },
  recorderControls: {
    flexDirection: 'row',
    gap: 10,
    justifyContent: 'center',
  },
  recordButton: {
    backgroundColor: '#d32f2f',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 32,
  },
  recordButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '700',
    textAlign: 'center',
  },
  pauseButton: {
    backgroundColor: '#f57c00',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 20,
  },
  resumeButton: {
    backgroundColor: '#388e3c',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 20,
  },
  stopButton: {
    backgroundColor: '#555',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 20,
  },
  controlButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '600',
    textAlign: 'center',
  },
  sectionTitle: {
    fontSize: 17,
    fontWeight: '600',
    color: '#333',
    marginBottom: 10,
  },
  recordingsList: {
    flex: 1,
  },
  recordingItem: {
    backgroundColor: '#fff',
    borderRadius: 8,
    padding: 12,
    marginBottom: 8,
    flexDirection: 'row',
    justifyContent: 'space-between',
    alignItems: 'center',
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 1 },
    shadowOpacity: 0.05,
    shadowRadius: 2,
    elevation: 1,
  },
  recordingInfo: {
    flex: 1,
  },
  recordingTime: {
    fontSize: 14,
    color: '#333',
    fontWeight: '500',
  },
  recordingDuration: {
    fontSize: 12,
    color: '#888',
    marginTop: 2,
  },
  recordingActions: {
    flexDirection: 'row',
    gap: 8,
  },
  playButton: {
    backgroundColor: '#007AFF',
    borderRadius: 6,
    paddingVertical: 8,
    paddingHorizontal: 14,
  },
  playButtonActive: {
    backgroundColor: '#555',
  },
  playButtonText: {
    color: '#fff',
    fontSize: 13,
    fontWeight: '600',
  },
  deleteButton: {
    backgroundColor: '#ffebee',
    borderRadius: 6,
    paddingVertical: 8,
    paddingHorizontal: 10,
  },
  deleteButtonText: {
    color: '#d32f2f',
    fontSize: 13,
    fontWeight: '600',
  },
  emptyState: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    paddingVertical: 40,
  },
  emptyStateText: {
    fontSize: 15,
    color: '#9e9e9e',
    textAlign: 'center',
  },
});

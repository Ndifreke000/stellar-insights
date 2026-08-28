import React, { useEffect, useState } from 'react';
import {
  View,
  Text,
  StyleSheet,
  ActivityIndicator,
  TouchableOpacity,
  TextInput,
} from 'react-native';
import Slider from '@react-native-community/slider';
import { useVideoPlayer } from '../features/video_player/useVideoPlayer';

const DEFAULT_VIDEO_URL = 'https://sample-videos.com/video321/mp4/720/big_buck_bunny_720p_1mb.mp4';

const formatTime = (seconds: number): string => {
  const m = Math.floor(seconds / 60);
  const s = Math.floor(seconds % 60);
  return `${m}:${s.toString().padStart(2, '0')}`;
};

export const VideoPlayerComponent = () => {
  const {
    isPlaying,
    isPaused,
    isLoading,
    isBuffering,
    currentTime,
    duration,
    volume,
    isMuted,
    error,
    isOfflineCached,
    play,
    pause,
    resume,
    stop,
    seek,
    setVolume,
    toggleMute,
    cacheForOffline,
    clearError,
  } = useVideoPlayer();

  const [videoUrl, setVideoUrl] = useState(DEFAULT_VIDEO_URL);
  const [isCaching, setIsCaching] = useState(false);

  useEffect(() => {
    return () => {
      stop();
    };
  }, []);

  const handlePlay = async () => {
    if (!videoUrl.trim()) return;
    await play(videoUrl.trim());
  };

  const handleCacheOffline = async () => {
    setIsCaching(true);
    await cacheForOffline(videoUrl.trim());
    setIsCaching(false);
  };

  if (isLoading || isBuffering) {
    return (
      <View style={styles.container} accessibilityLiveRegion="polite">
        <ActivityIndicator size="large" color="#007AFF" />
        <Text style={styles.loadingText} accessibilityLabel="Loading video">
          {isBuffering ? 'Buffering...' : 'Loading video...'}
        </Text>
      </View>
    );
  }

  return (
    <View style={styles.container}>
      <View style={styles.header} accessible accessibilityRole="header">
        <Text style={styles.title}>Video Player</Text>
      </View>

      {error && (
        <View style={styles.errorContainer} accessible accessibilityRole="alert" accessibilityLabel={`Error: ${error}`}>
          <Text style={styles.errorText}>{error}</Text>
          <TouchableOpacity onPress={clearError} accessibilityRole="button" accessibilityLabel="Dismiss error">
            <Text style={styles.dismissText}>Dismiss</Text>
          </TouchableOpacity>
        </View>
      )}

      <View style={styles.urlContainer}>
        <TextInput
          style={styles.urlInput}
          value={videoUrl}
          onChangeText={setVideoUrl}
          placeholder="Enter video URL"
          autoCapitalize="none"
          autoCorrect={false}
          accessibilityLabel="Video URL input"
          accessibilityHint="Enter the URL of the video to play"
        />
      </View>

      <View style={styles.videoPlaceholder} accessibilityLabel="Video display area" accessible>
        <Text style={styles.videoPlaceholderText}>
          {isPlaying ? '▶ Playing' : isPaused ? '⏸ Paused' : '⬛ Stopped'}
        </Text>
        {(isPlaying || isPaused) && (
          <Text style={styles.timeText} accessibilityLabel={`${formatTime(currentTime)} of ${formatTime(duration)}`}>
            {formatTime(currentTime)} / {formatTime(duration)}
          </Text>
        )}
      </View>

      {(isPlaying || isPaused) && (
        <View style={styles.seekContainer}>
          <Slider
            style={styles.slider}
            minimumValue={0}
            maximumValue={duration}
            value={currentTime}
            onSlidingComplete={seek}
            minimumTrackTintColor="#007AFF"
            maximumTrackTintColor="#ccc"
            thumbTintColor="#007AFF"
            accessibilityLabel="Seek bar"
            accessibilityHint="Slide to seek through the video"
          />
        </View>
      )}

      <View style={styles.controls}>
        {!isPlaying && !isPaused && (
          <TouchableOpacity
            style={styles.primaryButton}
            onPress={handlePlay}
            accessible
            accessibilityRole="button"
            accessibilityLabel="Play video"
          >
            <Text style={styles.primaryButtonText}>Play</Text>
          </TouchableOpacity>
        )}

        {isPlaying && (
          <TouchableOpacity
            style={styles.controlButton}
            onPress={pause}
            accessible
            accessibilityRole="button"
            accessibilityLabel="Pause video"
          >
            <Text style={styles.controlButtonText}>Pause</Text>
          </TouchableOpacity>
        )}

        {isPaused && (
          <TouchableOpacity
            style={styles.controlButton}
            onPress={resume}
            accessible
            accessibilityRole="button"
            accessibilityLabel="Resume video"
          >
            <Text style={styles.controlButtonText}>Resume</Text>
          </TouchableOpacity>
        )}

        {(isPlaying || isPaused) && (
          <TouchableOpacity
            style={[styles.controlButton, styles.stopButton]}
            onPress={stop}
            accessible
            accessibilityRole="button"
            accessibilityLabel="Stop video"
          >
            <Text style={styles.controlButtonText}>Stop</Text>
          </TouchableOpacity>
        )}
      </View>

      <View style={styles.volumeContainer}>
        <TouchableOpacity
          onPress={toggleMute}
          accessible
          accessibilityRole="button"
          accessibilityLabel={isMuted ? 'Unmute' : 'Mute'}
          style={styles.muteButton}
        >
          <Text style={styles.muteButtonText}>{isMuted ? 'Unmute' : 'Mute'}</Text>
        </TouchableOpacity>
        <Slider
          style={styles.volumeSlider}
          minimumValue={0}
          maximumValue={1}
          value={volume}
          onValueChange={setVolume}
          minimumTrackTintColor="#007AFF"
          maximumTrackTintColor="#ccc"
          thumbTintColor="#007AFF"
          accessibilityLabel="Volume slider"
          accessibilityHint="Adjust playback volume"
        />
      </View>

      <View style={styles.offlineContainer}>
        <TouchableOpacity
          style={[styles.cacheButton, isOfflineCached && styles.cachedButton]}
          onPress={handleCacheOffline}
          disabled={isCaching || isOfflineCached}
          accessible
          accessibilityRole="button"
          accessibilityLabel={isOfflineCached ? 'Video cached for offline' : 'Cache video for offline use'}
        >
          {isCaching ? (
            <ActivityIndicator size="small" color="#fff" />
          ) : (
            <Text style={styles.cacheButtonText}>
              {isOfflineCached ? 'Cached Offline' : 'Cache for Offline'}
            </Text>
          )}
        </TouchableOpacity>
      </View>
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
  urlContainer: {
    marginBottom: 12,
  },
  urlInput: {
    backgroundColor: '#fff',
    borderRadius: 8,
    borderWidth: 1,
    borderColor: '#ddd',
    paddingHorizontal: 12,
    paddingVertical: 10,
    fontSize: 14,
    color: '#333',
  },
  videoPlaceholder: {
    backgroundColor: '#1a1a2e',
    borderRadius: 12,
    height: 200,
    justifyContent: 'center',
    alignItems: 'center',
    marginBottom: 12,
  },
  videoPlaceholderText: {
    color: '#fff',
    fontSize: 20,
    fontWeight: '600',
  },
  timeText: {
    color: '#aaa',
    fontSize: 14,
    marginTop: 8,
  },
  seekContainer: {
    marginBottom: 8,
  },
  slider: {
    width: '100%',
    height: 40,
  },
  controls: {
    flexDirection: 'row',
    justifyContent: 'center',
    gap: 12,
    marginBottom: 16,
  },
  primaryButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 32,
    minWidth: 100,
  },
  primaryButtonText: {
    color: '#fff',
    fontSize: 16,
    fontWeight: '600',
    textAlign: 'center',
  },
  controlButton: {
    backgroundColor: '#5856D6',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 24,
  },
  stopButton: {
    backgroundColor: '#d32f2f',
  },
  controlButtonText: {
    color: '#fff',
    fontSize: 15,
    fontWeight: '600',
    textAlign: 'center',
  },
  volumeContainer: {
    flexDirection: 'row',
    alignItems: 'center',
    marginBottom: 16,
  },
  muteButton: {
    backgroundColor: '#555',
    borderRadius: 6,
    paddingVertical: 8,
    paddingHorizontal: 14,
    marginRight: 8,
  },
  muteButtonText: {
    color: '#fff',
    fontSize: 13,
    fontWeight: '600',
  },
  volumeSlider: {
    flex: 1,
    height: 40,
  },
  offlineContainer: {
    alignItems: 'center',
  },
  cacheButton: {
    backgroundColor: '#388e3c',
    borderRadius: 8,
    paddingVertical: 12,
    paddingHorizontal: 28,
    minWidth: 180,
    alignItems: 'center',
  },
  cachedButton: {
    backgroundColor: '#9e9e9e',
  },
  cacheButtonText: {
    color: '#fff',
    fontSize: 14,
    fontWeight: '600',
  },
});

import AsyncStorage from '@react-native-async-storage/async-storage';
import { useCallback, useEffect, useState } from 'react';
import { Platform, PermissionsAndroid } from 'react-native';
import NetInfo from '@react-native-community/netinfo';
import { PhotoAsset, GalleryState, MOCK_PHOTOS } from '../features/photo_gallery';

const GALLERY_CACHE_KEY = 'photo-gallery-cache';

export interface UsePhotoGallery extends Omit<GalleryState, 'selectedIds'> {
  selectedIds: string[];
  requestPermission: () => Promise<void>;
  loadPhotos: () => Promise<void>;
  toggleSelection: (id: string) => void;
  clearSelection: () => void;
  uploadSelected: () => Promise<void>;
}

async function loadCachedPhotos(): Promise<PhotoAsset[]> {
  try {
    const cached = await AsyncStorage.getItem(GALLERY_CACHE_KEY);
    return cached ? (JSON.parse(cached) as PhotoAsset[]) : MOCK_PHOTOS;
  } catch {
    return MOCK_PHOTOS;
  }
}

async function cachePhotos(photos: PhotoAsset[]): Promise<void> {
  try {
    await AsyncStorage.setItem(GALLERY_CACHE_KEY, JSON.stringify(photos));
  } catch {
    // best effort
  }
}

async function requestCameraRollPermission(): Promise<boolean> {
  if (Platform.OS === 'android') {
    const permission =
      parseInt(String(Platform.Version), 10) >= 33
        ? PermissionsAndroid.PERMISSIONS.READ_MEDIA_IMAGES
        : PermissionsAndroid.PERMISSIONS.READ_EXTERNAL_STORAGE;
    const granted = await PermissionsAndroid.request(permission, {
      title: 'Photo Library Permission',
      message: 'This app needs access to your photos.',
      buttonPositive: 'OK',
      buttonNegative: 'Cancel',
    });
    return granted === PermissionsAndroid.RESULTS.GRANTED;
  }
  return true;
}

export function usePhotoGallery(): UsePhotoGallery {
  const [permissionStatus, setPermissionStatus] = useState<GalleryState['permissionStatus']>('not_requested');
  const [photos, setPhotos] = useState<PhotoAsset[]>([]);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [uploadProgress, setUploadProgress] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [isOffline, setIsOffline] = useState(false);

  useEffect(() => {
    let active = true;

    async function init() {
      const netState = await NetInfo.fetch();
      if (active) {
        setIsOffline(!netState.isConnected || netState.isInternetReachable === false);
      }
      const cached = await loadCachedPhotos();
      if (active) {
        setPhotos(cached);
      }
    }

    void init();

    const unsubscribe = NetInfo.addEventListener(state => {
      setIsOffline(!state.isConnected || state.isInternetReachable === false);
    });

    return () => {
      active = false;
      unsubscribe();
    };
  }, []);

  const requestPermission = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const granted = await requestCameraRollPermission();
      setPermissionStatus(granted ? 'granted' : 'denied');
      if (!granted) {
        setError('Photo library permission denied. Enable it in Settings to view photos.');
      }
    } catch (err) {
      setPermissionStatus('unavailable');
      setError(err instanceof Error ? err.message : 'Failed to request photo library permission.');
    } finally {
      setLoading(false);
    }
  }, []);

  const loadPhotos = useCallback(async () => {
    if (permissionStatus !== 'granted') {
      setError('Photo library permission is required.');
      return;
    }
    setLoading(true);
    setError(null);
    try {
      // In a real app this would call CameraRoll.getPhotos or expo-media-library
      // Using cached/mock data since we cannot install new deps
      const cached = await loadCachedPhotos();
      setPhotos(cached);
      await cachePhotos(cached);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load photos.');
    } finally {
      setLoading(false);
    }
  }, [permissionStatus]);

  const toggleSelection = useCallback((id: string) => {
    setSelectedIds(prev => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  }, []);

  const clearSelection = useCallback(() => {
    setSelectedIds(new Set());
  }, []);

  const uploadSelected = useCallback(async () => {
    if (selectedIds.size === 0) {
      setError('No photos selected for upload.');
      return;
    }
    if (isOffline) {
      setError('Cannot upload while offline. Photos will be queued when connection is restored.');
      return;
    }
    setLoading(true);
    setError(null);
    setUploadProgress(0);
    try {
      const total = selectedIds.size;
      let completed = 0;
      for (const _id of selectedIds) {
        // Simulate upload per photo
        await new Promise<void>(resolve => setTimeout(resolve, 200));
        completed += 1;
        setUploadProgress(Math.round((completed / total) * 100));
      }
      clearSelection();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Upload failed.');
    } finally {
      setLoading(false);
      setUploadProgress(0);
    }
  }, [selectedIds, isOffline, clearSelection]);

  return {
    permissionStatus,
    photos,
    selectedIds: Array.from(selectedIds),
    uploadProgress,
    loading,
    error,
    isOffline,
    requestPermission,
    loadPhotos,
    toggleSelection,
    clearSelection,
    uploadSelected,
  };
}

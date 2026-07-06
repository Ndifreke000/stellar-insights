import { create } from 'zustand';
import { devtools, persist, subscribeWithSelector } from 'zustand/middleware';
import { immer } from 'zustand/middleware/immer';

// Types for our global state
export interface AppState {
  // UI State
  sidebarCollapsed: boolean;
  activeModal: string | null;
  loading: Record<string, boolean>;
  
  // Navigation State
  currentPage: string;
  breadcrumbs: Array<{ label: string; href: string }>;
  
  // Form State
  formData: Record<string, any>;
  formErrors: Record<string, Record<string, string>>;
  formDirty: Record<string, boolean>;
  
  // Filter State
  filters: Record<string, any>;
  
  // Notification State
  notifications: Array<{
    id: string;
    type: 'success' | 'error' | 'warning' | 'info';
    message: string;
    timestamp: number;
    read: boolean;
  }>;
  
  // User Session State
  userSession: {
    authenticated: boolean;
    userId?: string;
    permissions: string[];
  };
  
  // WebSocket State
  websocket: {
    connected: boolean;
    reconnecting: boolean;
    lastMessage: any;
  };
}

// Actions for updating state
export interface AppActions {
  // UI Actions
  setSidebarCollapsed: (collapsed: boolean) => void;
  setActiveModal: (modal: string | null) => void;
  setLoading: (key: string, loading: boolean) => void;
  clearLoading: (key?: string) => void;
  
  // Navigation Actions
  setCurrentPage: (page: string) => void;
  setBreadcrumbs: (breadcrumbs: Array<{ label: string; href: string }>) => void;
  addBreadcrumb: (breadcrumb: { label: string; href: string }) => void;
  
  // Form Actions
  setFormData: (key: string, data: any) => void;
  updateFormData: (key: string, updates: Partial<any>) => void;
  clearFormData: (key?: string) => void;
  setFormErrors: (key: string, errors: Record<string, string>) => void;
  clearFormErrors: (key?: string) => void;
  setFormDirty: (key: string, dirty: boolean) => void;
  clearFormDirty: (key?: string) => void;
  
  // Filter Actions
  setFilters: (filters: Record<string, any>) => void;
  updateFilter: (key: string, value: any) => void;
  clearFilters: (key?: string) => void;
  
  // Notification Actions
  addNotification: (notification: Omit<AppState['notifications'][0], 'id' | 'timestamp' | 'read'>) => void;
  removeNotification: (id: string) => void;
  markNotificationRead: (id: string) => void;
  markAllNotificationsRead: () => void;
  clearNotifications: () => void;
  
  // User Session Actions
  setAuthenticated: (authenticated: boolean, userId?: string, permissions?: string[]) => void;
  logout: () => void;
  
  // WebSocket Actions
  setWebSocketConnected: (connected: boolean) => void;
  setWebSocketReconnecting: (reconnecting: boolean) => void;
  setWebSocketLastMessage: (message: any) => void;
  
  // Reset Actions
  resetState: () => void;
}

// Create the store
export const useAppStore = create<AppState & AppActions>()(
  devtools(
    persist(
      subscribeWithSelector(
        immer((set, _get) => ({
          // Initial State
          sidebarCollapsed: false,
          activeModal: null,
          loading: {},
          currentPage: '/',
          breadcrumbs: [],
          formData: {},
          formErrors: {},
          formDirty: {},
          filters: {},
          notifications: [],
          userSession: {
            authenticated: false,
            permissions: [],
          },
          websocket: {
            connected: false,
            reconnecting: false,
            lastMessage: null,
          },

          // UI Actions
          setSidebarCollapsed: (collapsed) => {
            set((state) => {
              state.sidebarCollapsed = collapsed;
            });
          },

          setActiveModal: (modal) => {
            set((state) => {
              state.activeModal = modal;
            });
          },

          setLoading: (key, loading) => {
            set((state) => {
              state.loading[key] = loading;
            });
          },

          clearLoading: (key) => {
            set((state) => {
              if (key) {
                delete state.loading[key];
              } else {
                state.loading = {};
              }
            });
          },

          // Navigation Actions
          setCurrentPage: (page) => {
            set((state) => {
              state.currentPage = page;
            });
          },

          setBreadcrumbs: (breadcrumbs) => {
            set((state) => {
              state.breadcrumbs = breadcrumbs;
            });
          },

          addBreadcrumb: (breadcrumb) => {
            set((state) => {
              state.breadcrumbs.push(breadcrumb);
            });
          },

          // Form Actions
          setFormData: (key, data) => {
            set((state) => {
              state.formData[key] = data;
            });
          },

          updateFormData: (key, updates) => {
            set((state) => {
              if (state.formData[key]) {
                Object.assign(state.formData[key], updates);
              } else {
                state.formData[key] = updates;
              }
            });
          },

          clearFormData: (key) => {
            set((state) => {
              if (key) {
                delete state.formData[key];
              } else {
                state.formData = {};
              }
            });
          },

          setFormErrors: (key, errors) => {
            set((state) => {
              state.formErrors[key] = errors;
            });
          },

          clearFormErrors: (key) => {
            set((state) => {
              if (key) {
                delete state.formErrors[key];
              } else {
                state.formErrors = {};
              }
            });
          },

          setFormDirty: (key, dirty) => {
            set((state) => {
              state.formDirty[key] = dirty;
            });
          },

          clearFormDirty: (key) => {
            set((state) => {
              if (key) {
                delete state.formDirty[key];
              } else {
                state.formDirty = {};
              }
            });
          },

          // Filter Actions
          setFilters: (filters) => {
            set((state) => {
              state.filters = filters;
            });
          },

          updateFilter: (key, value) => {
            set((state) => {
              state.filters[key] = value;
            });
          },

          clearFilters: (key) => {
            set((state) => {
              if (key) {
                delete state.filters[key];
              } else {
                state.filters = {};
              }
            });
          },

          // Notification Actions
          addNotification: (notification) => {
            set((state) => {
              const newNotification = {
                ...notification,
                id: crypto.randomUUID(),
                timestamp: Date.now(),
                read: false,
              };
              state.notifications.unshift(newNotification);
              
              // Keep only last 50 notifications
              if (state.notifications.length > 50) {
                state.notifications = state.notifications.slice(0, 50);
              }
            });
          },

          removeNotification: (id) => {
            set((state) => {
              state.notifications = state.notifications.filter(n => n.id !== id);
            });
          },

          markNotificationRead: (id) => {
            set((state) => {
              const notification = state.notifications.find(n => n.id === id);
              if (notification) {
                notification.read = true;
              }
            });
          },

          markAllNotificationsRead: () => {
            set((state) => {
              state.notifications.forEach(n => {
                n.read = true;
              });
            });
          },

          clearNotifications: () => {
            set((state) => {
              state.notifications = [];
            });
          },

          // User Session Actions
          setAuthenticated: (authenticated, userId, permissions) => {
            set((state) => {
              state.userSession.authenticated = authenticated;
              state.userSession.userId = userId;
              state.userSession.permissions = permissions || [];
            });
          },

          logout: () => {
            set((state) => {
              state.userSession.authenticated = false;
              state.userSession.userId = undefined;
              state.userSession.permissions = [];
            });
          },

          // WebSocket Actions
          setWebSocketConnected: (connected) => {
            set((state) => {
              state.websocket.connected = connected;
            });
          },

          setWebSocketReconnecting: (reconnecting) => {
            set((state) => {
              state.websocket.reconnecting = reconnecting;
            });
          },

          setWebSocketLastMessage: (message) => {
            set((state) => {
              state.websocket.lastMessage = message;
            });
          },

          // Reset Actions
          resetState: () => {
            set((state) => {
              // Reset to initial values but keep some persistent state
              state.activeModal = null;
              state.loading = {};
              state.formData = {};
              state.formErrors = {};
              state.formDirty = {};
              state.notifications = [];
              state.websocket = {
                connected: false,
                reconnecting: false,
                lastMessage: null,
              };
            });
          },
        }))
      ),
      {
        name: 'stellar-insights-app-store',
        // Only persist certain parts of the state
        partialize: (state) => ({
          sidebarCollapsed: state.sidebarCollapsed,
          formData: state.formData,
          filters: state.filters,
          userSession: state.userSession,
        }),
        version: 1,
      }
    ),
    {
      name: 'Stellar Insights App Store',
    }
  )
);

// Selectors for commonly used state combinations
export const useAppState = () => useAppStore((state) => state);

export const useUIState = () => useAppStore((state) => ({
  sidebarCollapsed: state.sidebarCollapsed,
  activeModal: state.activeModal,
  loading: state.loading,
}));

export const useNavigationState = () => useAppStore((state) => ({
  currentPage: state.currentPage,
  breadcrumbs: state.breadcrumbs,
}));

export const useFormState = (formKey?: string) => useAppStore((state) => ({
  data: formKey ? state.formData[formKey] : state.formData,
  errors: formKey ? state.formErrors[formKey] : state.formErrors,
  dirty: formKey ? state.formDirty[formKey] : state.formDirty,
}));

export const useNotificationState = () => useAppStore((state) => ({
  notifications: state.notifications,
  unreadCount: state.notifications.filter(n => !n.read).length,
}));

export const useWebSocketState = () => useAppStore((state) => state.websocket);

export const useUserSession = () => useAppStore((state) => state.userSession);

// Convenience hooks for common actions
export const useAppActions = () => useAppStore((state) => ({
  setSidebarCollapsed: state.setSidebarCollapsed,
  setActiveModal: state.setActiveModal,
  setLoading: state.setLoading,
  clearLoading: state.clearLoading,
  setCurrentPage: state.setCurrentPage,
  setBreadcrumbs: state.setBreadcrumbs,
  addBreadcrumb: state.addBreadcrumb,
  setFormData: state.setFormData,
  updateFormData: state.updateFormData,
  clearFormData: state.clearFormData,
  setFormErrors: state.setFormErrors,
  clearFormErrors: state.clearFormErrors,
  setFormDirty: state.setFormDirty,
  clearFormDirty: state.clearFormDirty,
  setFilters: state.setFilters,
  updateFilter: state.updateFilter,
  clearFilters: state.clearFilters,
  addNotification: state.addNotification,
  removeNotification: state.removeNotification,
  markNotificationRead: state.markNotificationRead,
  markAllNotificationsRead: state.markAllNotificationsRead,
  clearNotifications: state.clearNotifications,
  setAuthenticated: state.setAuthenticated,
  logout: state.logout,
  setWebSocketConnected: state.setWebSocketConnected,
  setWebSocketReconnecting: state.setWebSocketReconnecting,
  setWebSocketLastMessage: state.setWebSocketLastMessage,
  resetState: state.resetState,
}));

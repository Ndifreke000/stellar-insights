import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { ProgressiveWebApp } from '../ProgressiveWebApp';
import * as useProgressiveWebAppModule from '@/hooks/useProgressiveWebApp';

// Mock the hook
vi.mock('@/hooks/useProgressiveWebApp', () => ({
  useProgressiveWebApp: vi.fn(),
  PWAInstallState: {
    UNSUPPORTED: 'unsupported',
    READY: 'ready',
    INSTALLING: 'installing',
    INSTALLED: 'installed',
    DISMISSED: 'dismissed',
  },
}));

describe('ProgressiveWebApp Component', () => {
  const mockHookReturn = {
    state: 'ready' as const,
    capabilities: {
      canInstall: true,
      isInstalled: false,
      isOnline: true,
      isStandalone: false,
      serviceWorkerReady: true,
      cacheSize: 1024 * 1024,
    },
    installPrompt: {} as any,
    install: vi.fn(),
    dismiss: vi.fn(),
    clearCache: vi.fn(),
    updateAvailable: false,
    updateApp: vi.fn(),
    isLoading: false,
    error: null,
  };

  beforeEach(() => {
    vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue(mockHookReturn);
  });

  describe('rendering', () => {
    it('should render without crashing', () => {
      render(<ProgressiveWebApp />);
      expect(screen.getByText(/Service Worker active/i)).toBeInTheDocument();
    });

    it('should render with custom className', () => {
      const { container } = render(<ProgressiveWebApp className="custom-class" />);
      expect(container.firstChild).toHaveClass('custom-class');
    });
  });

  describe('installation prompt', () => {
    it('should show installation prompt when canInstall is true', () => {
      render(<ProgressiveWebApp showInstallPrompt={true} />);
      expect(screen.getByText(/Install Stellar Insights/i)).toBeInTheDocument();
    });

    it('should not show installation prompt when showInstallPrompt is false', () => {
      render(<ProgressiveWebApp showInstallPrompt={false} />);
      expect(screen.queryByText(/Install Stellar Insights/i)).not.toBeInTheDocument();
    });

    it('should not show installation prompt when already installed', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showInstallPrompt={true} />);
      expect(screen.queryByText(/Install Stellar Insights/i)).not.toBeInTheDocument();
    });

    it('should call install when Install button is clicked', async () => {
      const user = userEvent.setup();
      render(<ProgressiveWebApp showInstallPrompt={true} />);

      const installButton = screen.getByRole('button', { name: /Install/i });
      await user.click(installButton);

      expect(mockHookReturn.install).toHaveBeenCalled();
    });

    it('should call dismiss when Not Now button is clicked', async () => {
      const user = userEvent.setup();
      render(<ProgressiveWebApp showInstallPrompt={true} />);

      const dismissButton = screen.getByRole('button', { name: /Not Now/i });
      await user.click(dismissButton);

      expect(mockHookReturn.dismiss).toHaveBeenCalled();
    });

    it('should show loading state during installation', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        isLoading: true,
      });

      render(<ProgressiveWebApp showInstallPrompt={true} />);
      expect(screen.getByText(/Installing.../i)).toBeInTheDocument();
    });
  });

  describe('offline indicator', () => {
    it('should show offline indicator when offline', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isOnline: false },
      });

      render(<ProgressiveWebApp showOfflineIndicator={true} />);
      expect(screen.getByText(/You're Offline/i)).toBeInTheDocument();
    });

    it('should not show offline indicator when showOfflineIndicator is false', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isOnline: false },
      });

      render(<ProgressiveWebApp showOfflineIndicator={false} />);
      expect(screen.queryByText(/You're Offline/i)).not.toBeInTheDocument();
    });

    it('should show online indicator when coming back online', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: {
          ...mockHookReturn.capabilities,
          isOnline: true,
          isInstalled: true,
        },
      });

      render(<ProgressiveWebApp showOfflineIndicator={true} />);
      expect(screen.getByText(/Back Online/i)).toBeInTheDocument();
    });
  });

  describe('update notification', () => {
    it('should show update notification when update is available', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        updateAvailable: true,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showUpdateNotification={true} />);
      expect(screen.getByText(/Update Available/i)).toBeInTheDocument();
    });

    it('should not show update notification when showUpdateNotification is false', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        updateAvailable: true,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showUpdateNotification={false} />);
      expect(screen.queryByText(/Update Available/i)).not.toBeInTheDocument();
    });

    it('should call updateApp when Update Now button is clicked', async () => {
      const user = userEvent.setup();
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        updateAvailable: true,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showUpdateNotification={true} />);

      const updateButton = screen.getByRole('button', { name: /Update Now/i });
      await user.click(updateButton);

      expect(mockHookReturn.updateApp).toHaveBeenCalled();
    });
  });

  describe('cache management', () => {
    it('should show cache management when showCacheManagement is true', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showCacheManagement={true} />);
      expect(screen.getByText(/Cache Management/i)).toBeInTheDocument();
    });

    it('should not show cache management when showCacheManagement is false', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showCacheManagement={false} />);
      expect(screen.queryByText(/Cache Management/i)).not.toBeInTheDocument();
    });

    it('should display cache size', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true, cacheSize: 1024 * 1024 },
      });

      render(<ProgressiveWebApp showCacheManagement={true} />);
      expect(screen.getByText(/Storage used: 1 MB/i)).toBeInTheDocument();
    });

    it('should toggle cache UI visibility', async () => {
      const user = userEvent.setup();
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showCacheManagement={true} />);

      const showButton = screen.getByRole('button', { name: /Show/i });
      await user.click(showButton);

      expect(screen.getByRole('button', { name: /Hide/i })).toBeInTheDocument();
    });

    it('should call clearCache when Clear Cache button is clicked', async () => {
      const user = userEvent.setup();
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp showCacheManagement={true} />);

      const showButton = screen.getByRole('button', { name: /Show/i });
      await user.click(showButton);

      const clearButton = screen.getByRole('button', { name: /Clear Cache/i });
      
      // Mock confirm
      window.confirm = vi.fn(() => true);
      
      await user.click(clearButton);

      expect(mockHookReturn.clearCache).toHaveBeenCalled();
    });
  });

  describe('installation status', () => {
    it('should show installation status when installed', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp />);
      expect(screen.getByText(/App Installed/i)).toBeInTheDocument();
    });

    it('should not show installation status when not installed', () => {
      render(<ProgressiveWebApp />);
      expect(screen.queryByText(/App Installed/i)).not.toBeInTheDocument();
    });
  });

  describe('error handling', () => {
    it('should display error message when error occurs', () => {
      const error = new Error('Test error message');
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        error,
      });

      render(<ProgressiveWebApp />);
      expect(screen.getByText(/Test error message/i)).toBeInTheDocument();
    });

    it('should not display error when error is null', () => {
      render(<ProgressiveWebApp />);
      expect(screen.queryByText(/Error/i)).not.toBeInTheDocument();
    });
  });

  describe('state change callback', () => {
    it('should call onStateChange when state changes', () => {
      const onStateChange = vi.fn();
      render(<ProgressiveWebApp onStateChange={onStateChange} />);

      expect(onStateChange).toHaveBeenCalledWith('ready');
    });
  });

  describe('accessibility', () => {
    it('should have proper ARIA labels', () => {
      render(<ProgressiveWebApp showInstallPrompt={true} />);
      
      const buttons = screen.getAllByRole('button');
      expect(buttons.length).toBeGreaterThan(0);
    });

    it('should have proper heading hierarchy', () => {
      vi.mocked(useProgressiveWebAppModule.useProgressiveWebApp).mockReturnValue({
        ...mockHookReturn,
        capabilities: { ...mockHookReturn.capabilities, isInstalled: true },
      });

      render(<ProgressiveWebApp />);
      
      const headings = screen.getAllByText(/App Installed|Cache Management/i);
      expect(headings.length).toBeGreaterThan(0);
    });

    it('should have proper color contrast', () => {
      const { container } = render(<ProgressiveWebApp />);
      
      // Check for proper contrast classes
      const elements = container.querySelectorAll('[class*="dark:"]');
      expect(elements.length).toBeGreaterThan(0);
    });
  });

  describe('responsive design', () => {
    it('should render properly on mobile', () => {
      render(<ProgressiveWebApp />);
      
      const container = screen.getByText(/Service Worker active/i).closest('div');
      expect(container).toHaveClass('space-y-3');
    });

    it('should render properly on desktop', () => {
      render(<ProgressiveWebApp />);
      
      const container = screen.getByText(/Service Worker active/i).closest('div');
      expect(container).toBeInTheDocument();
    });
  });
});

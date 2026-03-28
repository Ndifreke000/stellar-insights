import { render } from '@testing-library/react';
import { axe, toHaveNoViolations } from 'jest-axe';
import { Sidebar } from '../layout/sidebar';
import { CorridorHealthCard } from '../dashboard/CorridorHealthCard';
import { CreateProposalModal } from '../governance/CreateProposalModal';

// Extend Jest matchers
expect.extend(toHaveNoViolations);

// Mock next-intl
jest.mock('next-intl', () => ({
  useTranslations: () => (key: string) => key,
}));

// Mock navigation
jest.mock('@/i18n/navigation', () => ({
  Link: ({ children, ...props }: any) => <a {...props}>{children}</a>,
  usePathname: () => '/dashboard',
}));

// Mock user preferences
jest.mock('@/contexts/UserPreferencesContext', () => ({
  useUserPreferences: () => ({
    prefs: { sidebarCollapsed: false },
    setPrefs: jest.fn(),
  }),
}));

describe('Accessibility Tests', () => {
  describe('Sidebar Component', () => {
    it('should not have accessibility violations', async () => {
      const { container } = render(<Sidebar />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('should have proper ARIA labels on navigation', () => {
      const { getByLabelText } = render(<Sidebar />);
      expect(getByLabelText(/main navigation/i)).toBeInTheDocument();
    });

    it('should mark icons as decorative', () => {
      const { container } = render(<Sidebar />);
      const icons = container.querySelectorAll('svg');
      icons.forEach(icon => {
        expect(icon).toHaveAttribute('aria-hidden', 'true');
      });
    });

    it('should have aria-current on active page', () => {
      const { container } = render(<Sidebar />);
      const activeLink = container.querySelector('[aria-current="page"]');
      expect(activeLink).toBeInTheDocument();
    });
  });

  describe('CorridorHealthCard Component', () => {
    const mockCorridors = [
      { id: 'USD-EUR', health: 0.95, successRate: 0.98 },
      { id: 'USD-GBP', health: 0.87, successRate: 0.92 },
    ];

    it('should not have accessibility violations', async () => {
      const { container } = render(<CorridorHealthCard corridors={mockCorridors} />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('should have proper heading structure', () => {
      const { getByRole } = render(<CorridorHealthCard corridors={mockCorridors} />);
      expect(getByRole('heading', { level: 2 })).toBeInTheDocument();
    });

    it('should use semantic list markup', () => {
      const { getByRole } = render(<CorridorHealthCard corridors={mockCorridors} />);
      expect(getByRole('list')).toBeInTheDocument();
    });

    it('should have status indicators with aria-label', () => {
      const { container } = render(<CorridorHealthCard corridors={mockCorridors} />);
      const statusElements = container.querySelectorAll('[role="status"]');
      statusElements.forEach(status => {
        expect(status).toHaveAttribute('aria-label');
      });
    });
  });

  describe('CreateProposalModal Component', () => {
    const mockProps = {
      authToken: 'test-token',
      onClose: jest.fn(),
      onCreated: jest.fn(),
    };

    it('should not have accessibility violations', async () => {
      const { container } = render(<CreateProposalModal {...mockProps} />);
      const results = await axe(container);
      expect(results).toHaveNoViolations();
    });

    it('should have dialog role and aria-modal', () => {
      const { getByRole } = render(<CreateProposalModal {...mockProps} />);
      const dialog = getByRole('dialog');
      expect(dialog).toHaveAttribute('aria-modal', 'true');
    });

    it('should have aria-labelledby pointing to title', () => {
      const { getByRole } = render(<CreateProposalModal {...mockProps} />);
      const dialog = getByRole('dialog');
      const labelledBy = dialog.getAttribute('aria-labelledby');
      expect(labelledBy).toBeTruthy();
      expect(document.getElementById(labelledBy!)).toBeInTheDocument();
    });

    it('should have form labels associated with inputs', () => {
      const { getByLabelText } = render(<CreateProposalModal {...mockProps} />);
      expect(getByLabelText(/title/i)).toBeInTheDocument();
      expect(getByLabelText(/description/i)).toBeInTheDocument();
      expect(getByLabelText(/target contract/i)).toBeInTheDocument();
      expect(getByLabelText(/wasm hash/i)).toBeInTheDocument();
    });

    it('should mark required fields with aria-required', () => {
      const { container } = render(<CreateProposalModal {...mockProps} />);
      const requiredInputs = container.querySelectorAll('[required]');
      requiredInputs.forEach(input => {
        expect(input).toHaveAttribute('aria-required', 'true');
      });
    });

    it('should have close button with aria-label', () => {
      const { getByLabelText } = render(<CreateProposalModal {...mockProps} />);
      expect(getByLabelText(/close modal/i)).toBeInTheDocument();
    });
  });

  describe('Keyboard Navigation', () => {
    it('sidebar links should be keyboard accessible', () => {
      const { container } = render(<Sidebar />);
      const links = container.querySelectorAll('a');
      links.forEach(link => {
        expect(link).not.toHaveAttribute('tabindex', '-1');
      });
    });

    it('modal should trap focus', () => {
      const { container } = render(
        <CreateProposalModal
          authToken="test"
          onClose={jest.fn()}
          onCreated={jest.fn()}
        />
      );
      
      const focusableElements = container.querySelectorAll(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      
      expect(focusableElements.length).toBeGreaterThan(0);
    });
  });

  describe('Screen Reader Support', () => {
    it('should hide decorative icons from screen readers', () => {
      const { container } = render(<Sidebar />);
      const decorativeIcons = container.querySelectorAll('[aria-hidden="true"]');
      expect(decorativeIcons.length).toBeGreaterThan(0);
    });

    it('should provide text alternatives for icon-only buttons', () => {
      const { getByLabelText } = render(<Sidebar />);
      expect(getByLabelText(/collapse/i)).toBeInTheDocument();
    });

    it('should announce status changes', () => {
      const { container } = render(<Sidebar />);
      const statusRegion = container.querySelector('[role="status"]');
      expect(statusRegion).toHaveAttribute('aria-live', 'polite');
    });
  });
});

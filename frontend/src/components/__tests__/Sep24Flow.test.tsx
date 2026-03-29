import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { Sep24Flow } from '../Sep24Flow';
import * as validation from '../../lib/validation';
import * as sep24 from '../../../services/sep24';

// Mock services
jest.mock('../../../services/sep24');

const mockLoadAnchors = sep24.getSep24Anchors as jest.MockedFunction<typeof sep24.getSep24Anchors>;
const mockLoadInfo = sep24.getSep24Info as jest.MockedFunction<typeof sep24.getSep24Info>;

describe('Sep24Flow', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockLoadAnchors.mockResolvedValue({ anchors: [{ name: 'Test', transfer_server: 'https://test.com/sep24' }] });
  });

  it('renders without crashing', () => {
    render(<Sep24Flow />);
  });

  it('shows URL validation error on invalid URL', async () => {
    render(<Sep24Flow />);
    const urlInput = screen.getByPlaceholderText(/https:\/\/api.anchor.example\/sep24/i) as HTMLInputElement;
    fireEvent.change(urlInput, { target: { value: 'invalid-url' } });
    await waitFor(() => {
      expect(screen.getByText(/valid url/i)).toBeInTheDocument();
    });
    expect(urlInput.getAttribute('aria-invalid')).toBe('true');
  });

  it('shows amount validation error on invalid amount', async () => {
    render(<Sep24Flow />);
    // Select anchor to load info
    fireEvent.change(screen.getByRole('combobox'), { target: { value: 'https://test.com/sep24' } });
    await waitFor(() => screen.getByPlaceholderText('0.00'));
    const amountInput = screen.getByPlaceholderText('0.00') as HTMLInputElement;
    fireEvent.change(amountInput, { target: { value: '-1' } });
    await waitFor(() => {
      expect(screen.getByText(/positive number/i)).toBeInTheDocument();
    });
    expect(amountInput.getAttribute('aria-invalid')).toBe('true');
  });

  it('validates Stellar account correctly', async () => {
    render(<Sep24Flow />);
    // Assume account input available after form load
    const accountInput = screen.getByPlaceholderText('G...') as HTMLInputElement;
    fireEvent.change(accountInput, { target: { value: 'invalid' } });
    await waitFor(() => {
      expect(screen.getByText(/stellar account/i)).toBeInTheDocument();
    });
    fireEvent.change(accountInput, { target: { value: 'GABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890ABCDEFGHIJKL' } });
    await waitFor(() => {
      expect(screen.queryByText(/stellar account/i)).not.toBeInTheDocument();
    });
  });

  it('disables start button if validation fails', async () => {
    render(<Sep24Flow />);
    fireEvent.change(screen.getByPlaceholderText(/https:\/\/api.anchor.example\/sep24/i) as HTMLInputElement, { target: { value: 'invalid' } });
    await waitFor(() => {
      const button = screen.getByRole('button', { name: /start/i });
      expect(button).toBeDisabled();
    });
  });

  it('allows valid form submission', async () => {
    const mockStartFlow = jest.fn().mockResolvedValue({ url: 'https://interactive.com' });
    (sep24.startDepositInteractive as jest.Mock).mockResolvedValue(mockStartFlow());
    render(<Sep24Flow />);
    const urlInput = screen.getByPlaceholderText(/https:\/\/api.anchor.example\/sep24/i) as HTMLInputElement;
    fireEvent.change(urlInput, { target: { value: 'https://valid.com/sep24' } });
    fireEvent.click(screen.getByRole('button', { name: /start/i }));
    await waitFor(() => expect(mockStartFlow).toHaveBeenCalled());
  });
});

import { screen, fireEvent, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';
import { renderWithProviders } from '@/test-utils/render';
import { Sep24Flow } from '../Sep24Flow';
import * as sep24 from '../../services/sep24';

vi.mock('../../services/sep24');

const TRANSFER_SERVER = 'https://test.com/sep24';
const URL_PLACEHOLDER = /https:\/\/api.anchor.example\/sep24/i;

const mockAnchors = sep24.getSep24Anchors as MockedFunction<typeof sep24.getSep24Anchors>;
const mockInfo = sep24.getSep24Info as MockedFunction<typeof sep24.getSep24Info>;
const mockTransactions = sep24.getSep24Transactions as MockedFunction<
  typeof sep24.getSep24Transactions
>;

/** Type a transfer server URL and wait for the anchor's info to render the form. */
async function loadAnchorForm() {
  fireEvent.change(screen.getByPlaceholderText(URL_PLACEHOLDER), {
    target: { value: TRANSFER_SERVER },
  });
  return {
    amount: (await screen.findByPlaceholderText('0.00')) as HTMLInputElement,
    account: screen.getByPlaceholderText('G...') as HTMLInputElement,
  };
}

describe('Sep24Flow', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAnchors.mockResolvedValue({
      anchors: [{ name: 'Test', transfer_server: TRANSFER_SERVER }],
    } as Awaited<ReturnType<typeof sep24.getSep24Anchors>>);
    mockInfo.mockResolvedValue({
      deposit: { USDC: { enabled: true } },
      withdraw: { USDC: { enabled: true } },
    });
    mockTransactions.mockResolvedValue({
      transactions: [],
    } as Awaited<ReturnType<typeof sep24.getSep24Transactions>>);
  });

  it('renders the transfer server field before any anchor is chosen', () => {
    renderWithProviders(<Sep24Flow />);
    expect(screen.getByPlaceholderText(URL_PLACEHOLDER)).toBeInTheDocument();
    expect(screen.queryByPlaceholderText('0.00')).not.toBeInTheDocument();
  });

  it('flags an invalid transfer server URL', async () => {
    renderWithProviders(<Sep24Flow />);
    const urlInput = screen.getByPlaceholderText(URL_PLACEHOLDER);
    fireEvent.change(urlInput, { target: { value: 'invalid-url' } });

    await waitFor(() => expect(urlInput).toHaveAttribute('aria-invalid', 'true'));
    expect(mockInfo).not.toHaveBeenCalledWith(TRANSFER_SERVER);
  });

  it('shows the amount and account fields once the anchor info loads', async () => {
    renderWithProviders(<Sep24Flow />);
    const { amount, account } = await loadAnchorForm();

    expect(mockInfo).toHaveBeenCalledWith(TRANSFER_SERVER);
    expect(amount).toBeInTheDocument();
    expect(account).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /start deposit/i })).toBeInTheDocument();
  });

  it('rejects a negative amount', async () => {
    renderWithProviders(<Sep24Flow />);
    const { amount } = await loadAnchorForm();
    fireEvent.change(amount, { target: { value: '-1' } });

    expect(await screen.findByText(/positive number/i)).toBeInTheDocument();
    expect(amount).toHaveAttribute('aria-invalid', 'true');
  });

  it('keeps the start button disabled while the form is invalid', async () => {
    renderWithProviders(<Sep24Flow />);
    const { amount } = await loadAnchorForm();
    fireEvent.change(amount, { target: { value: '-1' } });

    await waitFor(() =>
      expect(screen.getByRole('button', { name: /start deposit/i })).toBeDisabled(),
    );
  });

  it('switches the action to a withdrawal', async () => {
    renderWithProviders(<Sep24Flow />);
    await loadAnchorForm();
    fireEvent.click(screen.getByRole('button', { name: /^withdraw$/i }));

    expect(await screen.findByRole('button', { name: /start withdrawal/i })).toBeInTheDocument();
  });
});

import React from 'react';
import { render, fireEvent } from '@testing-library/react-native';
import { InfiniteScroll } from '../InfiniteScroll';
import { useInfiniteScroll } from '@hooks/useInfiniteScroll';

jest.mock('@hooks/useInfiniteScroll', () => ({
  useInfiniteScroll: jest.fn(),
}));

const mockedUseInfiniteScroll = useInfiniteScroll as jest.MockedFunction<typeof useInfiniteScroll>;

describe('InfiniteScroll', () => {
  const state = {
    items: [{ id: 'insight-1', title: 'Insight 1', description: 'Loaded from page 1' }],
    page: 1,
    cursor: null,
    hasMore: true,
    isLoading: false,
    platformThreshold: 0.4,
    loadMore: jest.fn(),
    refresh: jest.fn(),
  };

  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseInfiniteScroll.mockReturnValue(state);
  });

  it('renders correctly', async () => {
    const { getByText } = await render(<InfiniteScroll />);

    expect(getByText('Infinite Scroll')).toBeTruthy();
    expect(getByText('Insight 1')).toBeTruthy();
  });

  it('loads more from the footer action', async () => {
    const { getByLabelText } = await render(<InfiniteScroll />);
    const button = getByLabelText('Load more infinite scroll results');

    await fireEvent.press(button);

    expect(state.loadMore).toHaveBeenCalled();
  });
});

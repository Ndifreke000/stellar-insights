import React from 'react';
import renderer, { act } from 'react-test-renderer';
import { Text } from 'react-native';
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

  it('renders correctly', () => {
    const tree = renderer.create(<InfiniteScroll />);
    const texts = tree.root.findAllByType(Text).map(node => node.props.children);

    expect(texts).toContain('Infinite Scroll');
    expect(texts).toContain('Insight 1');
  });

  it('loads more from the footer action', () => {
    const tree = renderer.create(<InfiniteScroll />);
    const button = tree.root.findByProps({ accessibilityLabel: 'Load more infinite scroll results' });

    act(() => {
      button.props.onPress();
    });

    expect(state.loadMore).toHaveBeenCalled();
  });
});

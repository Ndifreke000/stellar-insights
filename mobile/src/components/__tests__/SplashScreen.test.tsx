import React from 'react';
import renderer from 'react-test-renderer';
import { ActivityIndicator, Text } from 'react-native';
import { SplashScreen } from '../SplashScreen';
import { useSplashScreen } from '@hooks/useSplashScreen';

jest.mock('@hooks/useSplashScreen', () => ({
  useSplashScreen: jest.fn(),
}));

const mockedUseSplashScreen = useSplashScreen as jest.MockedFunction<typeof useSplashScreen>;

const baseResult = {
  status: 'loading' as const,
  error: null,
  isVisible: true,
  platformName: 'iOS',
};

describe('SplashScreen', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockedUseSplashScreen.mockReturnValue(baseResult);
  });

  it('renders correctly in loading state', () => {
    const tree = renderer.create(<SplashScreen />);
    const texts = tree.root.findAllByType(Text).map(n => n.props.children);

    expect(texts).toContain('PayRaider');
    expect(tree.root.findAllByType(ActivityIndicator)).toHaveLength(1);
  });

  it('shows error message in error state', () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'error',
      error: 'DB init failed',
      isVisible: false,
    });

    const tree = renderer.create(<SplashScreen />);
    const texts = tree.root.findAllByType(Text).map(n => n.props.children);

    expect(texts).toContain('DB init failed');
    expect(tree.root.findAllByType(ActivityIndicator)).toHaveLength(0);
  });

  it('shows fallback error message when error is null', () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'error',
      error: null,
      isVisible: false,
    });

    const tree = renderer.create(<SplashScreen />);
    const texts = tree.root.findAllByType(Text).map(n => n.props.children);

    expect(texts).toContain('Something went wrong. Please restart the app.');
  });

  it('hides spinner in ready state', () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'ready',
      isVisible: false,
    });

    const tree = renderer.create(<SplashScreen />);

    expect(tree.root.findAllByType(ActivityIndicator)).toHaveLength(0);
  });

  it('has accessibility label on container', () => {
    const tree = renderer.create(<SplashScreen />);
    const root = tree.toJSON() as renderer.ReactTestRendererJSON;

    expect(root.props.accessibilityLabel).toBe('Loading PayRaider');
  });

  it('has error accessibility label when in error state', () => {
    mockedUseSplashScreen.mockReturnValue({
      ...baseResult,
      status: 'error',
      error: 'Network error',
      isVisible: false,
    });

    const tree = renderer.create(<SplashScreen />);
    const root = tree.toJSON() as renderer.ReactTestRendererJSON;

    expect(root.props.accessibilityLabel).toBe('Initialization error: Network error');
  });
});

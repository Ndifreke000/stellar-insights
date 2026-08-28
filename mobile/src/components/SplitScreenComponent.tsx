import React, { ReactNode } from 'react';
import { View, StyleSheet, Platform, ViewStyle } from 'react-native';
import { useSplitScreen } from '../hooks/useSplitScreen';
import type { SplitScreenConfig } from '../features/split_screen/types';

interface SplitScreenComponentProps {
  primaryContent: ReactNode;
  secondaryContent?: ReactNode;
  config?: Partial<SplitScreenConfig>;
  dividerColor?: string;
  style?: ViewStyle;
  forceSplitMode?: 'full' | 'half' | 'one-third' | 'two-thirds' | 'slide-over';
}

export const SplitScreenComponent: React.FC<SplitScreenComponentProps> = ({
  primaryContent,
  secondaryContent,
  config,
  dividerColor = '#e0e0e0',
  style,
  forceSplitMode,
}) => {
  const split = useSplitScreen(config);
  const effectiveMode = forceSplitMode ?? split.mode;
  const showSecondary = effectiveMode !== 'full' && !!secondaryContent;

  return (
    <View style={[styles.container, style]} accessibilityLabel={showSecondary ? 'Split screen layout' : 'Single pane layout'}>
      <View style={[styles.pane, { flex: split.getPaneAFlex() }]} accessibilityLabel="Primary content pane">
        {primaryContent}
      </View>
      {showSecondary && (
        <View style={[styles.divider, { backgroundColor: dividerColor }]} accessibilityElementsHidden importantForAccessibility="no" />
      )}
      {showSecondary && (
        <View
          style={[styles.pane, { flex: split.getPaneBFlex() }, effectiveMode === 'slide-over' && styles.slideOver]}
          accessibilityLabel="Secondary content pane"
        >
          {secondaryContent}
        </View>
      )}
    </View>
  );
};

const styles = StyleSheet.create({
  container: { flex: 1, flexDirection: 'row' },
  pane: { overflow: 'hidden' },
  divider: {
    width: StyleSheet.hairlineWidth,
    ...Platform.select({
      ios: { shadowColor: '#000', shadowOffset: { width: 1, height: 0 }, shadowOpacity: 0.08 },
      android: { elevation: 1 },
    }),
  },
  slideOver: {
    position: 'absolute',
    right: 0,
    top: 0,
    bottom: 0,
    ...Platform.select({
      ios: { shadowColor: '#000', shadowOffset: { width: -2, height: 0 }, shadowOpacity: 0.15, shadowRadius: 8 },
      android: { elevation: 8 },
    }),
  },
});

export default SplitScreenComponent;

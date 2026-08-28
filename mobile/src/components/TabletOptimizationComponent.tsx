import React, { ReactNode } from 'react';
import { View, StyleSheet, ViewStyle, Platform } from 'react-native';
import { useTabletOptimization } from '../hooks/useTabletOptimization';
import type { TabletConfig } from '../features/tablet_optimization/types';

interface TabletOptimizationComponentProps {
  children: ReactNode;
  sidebar?: ReactNode;
  config?: Partial<TabletConfig>;
  style?: ViewStyle;
  forceTablet?: boolean;
}

export const TabletOptimizationComponent: React.FC<TabletOptimizationComponentProps> = ({
  children,
  sidebar,
  config,
  style,
  forceTablet,
}) => {
  const tablet = useTabletOptimization(config);
  const isTablet = forceTablet !== undefined ? forceTablet : tablet.isTablet;

  if (!isTablet) {
    return (
      <View style={[styles.singleColumn, style]} accessible accessibilityLabel="Main content">
        {children}
      </View>
    );
  }

  return (
    <View style={[styles.tabletContainer, style]} accessible accessibilityLabel="Tablet layout">
      {sidebar && (
        <View style={[styles.sidebar, { width: tablet.getSidebarWidth() }]} accessibilityLabel="Sidebar">
          {sidebar}
        </View>
      )}
      <View
        style={[styles.content, tablet.config.contentMaxWidth ? { maxWidth: tablet.config.contentMaxWidth } : undefined]}
        accessibilityLabel="Content area"
      >
        {children}
      </View>
    </View>
  );
};

const styles = StyleSheet.create({
  singleColumn: { flex: 1 },
  tabletContainer: { flex: 1, flexDirection: 'row' },
  sidebar: {
    borderRightWidth: StyleSheet.hairlineWidth,
    borderRightColor: '#e0e0e0',
    ...Platform.select({
      ios: { shadowColor: '#000', shadowOffset: { width: 1, height: 0 }, shadowOpacity: 0.05 },
      android: { elevation: 2 },
    }),
  },
  content: { flex: 1 },
});

export default TabletOptimizationComponent;

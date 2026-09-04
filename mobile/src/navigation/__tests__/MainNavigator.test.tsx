import fs from 'fs';
import path from 'path';

import React from 'react';
import { render } from '@testing-library/react-native';
import { NavigationContainer } from '@react-navigation/native';

import { MainNavigator } from '@navigation/MainNavigator';

const mainNavigatorSource = fs.readFileSync(
  path.join(__dirname, '../MainNavigator.tsx'),
  'utf8',
);

const TAB_ROUTE_NAMES = [
  'Dashboard',
  'Corridors',
  'Anchors',
  'NetworkSwitchDialog',
  'SearchFunctionality',
  'Settings',
] as const;

function mockScreen(label: string) {
  const React = require('react') as typeof import('react');
  const { Text } = require('react-native');
  return function MockScreen() {
    return React.createElement(Text, null, label);
  };
}

jest.mock('@components/DashboardScreen', () => ({
  DashboardScreen: mockScreen('Dashboard'),
}));

jest.mock('@screens/main/SettingsScreen', () => ({
  SettingsScreen: mockScreen('Settings'),
}));

jest.mock('@components/CorridorsList', () => ({
  CorridorsList: mockScreen('Corridors List'),
}));

jest.mock('@components/AnchorsList', () => ({
  AnchorsList: mockScreen('Anchors List'),
}));

jest.mock('@components/CorridorDetail', () => ({
  CorridorDetail: mockScreen('Corridor Detail'),
}));

jest.mock('@components/AnchorDetail', () => ({
  AnchorDetail: mockScreen('Anchor Detail'),
}));

jest.mock('@components/NetworkSwitchDialog', () => ({
  NetworkSwitchButton: mockScreen('Network Switch'),
}));

jest.mock('@components/SearchFunctionality', () => ({
  SearchFunctionality: mockScreen('Search'),
}));

function createTabInitialState(activeTab: (typeof TAB_ROUTE_NAMES)[number]) {
  const index = TAB_ROUTE_NAMES.indexOf(activeTab);

  return {
    stale: false,
    type: 'tab' as const,
    key: 'main-tabs',
    index,
    routeNames: [...TAB_ROUTE_NAMES],
    routes: TAB_ROUTE_NAMES.map(name => {
      if (name === 'Corridors') {
        return {
          name,
          key: `${name}-key`,
          state: {
            stale: false,
            type: 'stack' as const,
            key: 'corridors-stack',
            index: 0,
            routeNames: ['CorridorsList', 'CorridorDetail'],
            routes: [{ name: 'CorridorsList', key: 'CorridorsList-key' }],
          },
        };
      }

      if (name === 'Anchors') {
        return {
          name,
          key: `${name}-key`,
          state: {
            stale: false,
            type: 'stack' as const,
            key: 'anchors-stack',
            index: 0,
            routeNames: ['AnchorsList', 'AnchorDetail'],
            routes: [{ name: 'AnchorsList', key: 'AnchorsList-key' }],
          },
        };
      }

      return { name, key: `${name}-key` };
    }),
  };
}

describe('MainNavigator', () => {
  it('declares native stack navigators for corridors and anchors', () => {
    expect(mainNavigatorSource).toMatch(
      /const CorridorsStack = createNativeStackNavigator/,
    );
    expect(mainNavigatorSource).toMatch(
      /const AnchorsStack = createNativeStackNavigator/,
    );
  });

  it('renders the default tab navigator without throwing', () => {
    expect(() => {
      render(
        <NavigationContainer>
          <MainNavigator />
        </NavigationContainer>,
      );
    }).not.toThrow();
  });

  it('mounts the anchors stack when the Anchors tab is active', async () => {
    const { getByText } = await render(
      <NavigationContainer initialState={createTabInitialState('Anchors')}>
        <MainNavigator />
      </NavigationContainer>,
    );

    expect(getByText('Anchors List')).toBeTruthy();
  });

  it('mounts the corridors stack when the Corridors tab is active', async () => {
    const { getByText } = await render(
      <NavigationContainer initialState={createTabInitialState('Corridors')}>
        <MainNavigator />
      </NavigationContainer>,
    );

    expect(getByText('Corridors List')).toBeTruthy();
  });
});

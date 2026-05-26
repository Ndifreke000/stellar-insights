import React from 'react';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { DashboardScreen } from '@screens/main/DashboardScreen';
import { CorridorsScreen } from '@screens/main/CorridorsScreen';
import { AnchorsScreen } from '@screens/main/AnchorsScreen';
import { SettingsScreen } from '@screens/main/SettingsScreen';
import { SearchFunctionality } from '@components/SearchFunctionality';

export type MainTabParamList = {
  Dashboard: undefined;
  Corridors: undefined;
  Anchors: undefined;
  Search: undefined;
  Settings: undefined;
};

const Tab = createBottomTabNavigator<MainTabParamList>();

export function MainNavigator() {
  return (
    <Tab.Navigator screenOptions={{ headerShown: true }}>
      <Tab.Screen name="Dashboard" component={DashboardScreen} />
      <Tab.Screen name="Corridors" component={CorridorsScreen} />
      <Tab.Screen name="Anchors" component={AnchorsScreen} />
      <Tab.Screen name="Search" component={SearchFunctionality} />
      <Tab.Screen name="Settings" component={SettingsScreen} />
    </Tab.Navigator>
  );
}

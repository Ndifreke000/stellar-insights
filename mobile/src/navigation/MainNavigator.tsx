import React from 'react';
import { createBottomTabNavigator } from '@react-navigation/bottom-tabs';
import { createNativeStackNavigator } from '@react-navigation/native-stack';
import { View, Text } from 'react-native';
import { DashboardScreen } from '@screens/main/DashboardScreen';
import { CorridorsList } from '@components/CorridorsList';
import { AnchorsList } from '@components/AnchorsList';
import { SettingsScreen } from '@screens/main/SettingsScreen';
import { CorridorDetail } from '@components/CorridorDetail';
import { OfflineQueue } from '@components/OfflineQueue';
import { InfiniteScroll } from '@components/InfiniteScroll';
import { PullToRefresh } from '@components/PullToRefresh';
import { OfflineCaching } from '@components/OfflineCaching';
import { NetworkSwitchButton } from '@components/NetworkSwitchDialog';
import { SearchFunctionality } from '@components/SearchFunctionality';
import { IOSProjectSetup } from '@components/IOSProjectSetup';

export type CorridorsStackParamList = {
  CorridorsList: undefined;
  CorridorDetail: {
    corridorId: string;
  };
};

export type MainTabParamList = {
  Dashboard: undefined;
  Corridors: undefined;
  Anchors: undefined;
  OfflineQueue: undefined;
  InfiniteScroll: undefined;
  PullToRefresh: undefined;
  OfflineCaching: undefined;
  NetworkSwitchDialog: undefined;
  SearchFunctionality: undefined;
  IOSProjectSetup: undefined;
  Settings: undefined;
};

const Tab = createBottomTabNavigator<MainTabParamList>();
const CorridorsStack = createNativeStackNavigator<CorridorsStackParamList>();

function CorridorsNavigator() {
  return (
    <CorridorsStack.Navigator>
      <CorridorsStack.Screen
        name="CorridorsList"
        component={CorridorsList}
        options={{ title: 'Corridors', headerShown: false }}
      />
      <CorridorsStack.Screen
        name="CorridorDetail"
        component={CorridorDetail}
        options={{ title: 'Corridor Detail' }}
      />
    </CorridorsStack.Navigator>
  );
}

// Wrapper component for Network Switch
const NetworkSwitchScreen = () => {
  const [dialogVisible, setDialogVisible] = React.useState(true);

  return (
    <View style={{ flex: 1, justifyContent: 'center', alignItems: 'center' }}>
      <NetworkSwitchButton onPress={() => setDialogVisible(true)} />
    </View>
  );
};

// Wrapper component for Search Functionality
const SearchFunctionalityScreen = () => {
  const [searchData] = React.useState([
    {
      id: '1',
      name: 'Stellar Development Foundation',
      description: 'Official Stellar organization',
    },
    { id: '2', name: 'Stellar Lumens', description: 'Cryptocurrency token' },
    { id: '3', name: 'Stellar Protocol', description: 'Blockchain protocol' },
    { id: '4', name: 'Stellar Quest', description: 'Learning platform' },
    { id: '5', name: 'Stellar Anchor', description: 'Bridge between different networks' },
  ]);

  return (
    <SearchFunctionality
      data={searchData}
      renderItem={({ item }) => (
        <View>
          <Text style={{ fontSize: 14, fontWeight: '600', color: '#212121', marginBottom: 4 }}>
            {item.name}
          </Text>
          <Text style={{ fontSize: 12, color: '#666666' }}>{item.description}</Text>
        </View>
      )}
      placeholder="Search Stellar resources..."
    />
  );
};

export function MainNavigator() {
  return (
    <Tab.Navigator screenOptions={{ headerShown: true }}>
      <Tab.Screen name="Dashboard" component={DashboardScreen} />
      <Tab.Screen
        name="Corridors"
        component={CorridorsNavigator}
        options={{ headerShown: false }}
      />
      <Tab.Screen
        name="Anchors"
        component={AnchorsList}
        options={{ title: 'Anchors' }}
      />
      <Tab.Screen
        name="OfflineQueue"
        component={OfflineQueue}
        options={{ title: 'Offline Queue' }}
      />
      <Tab.Screen
        name="InfiniteScroll"
        component={InfiniteScroll}
        options={{ title: 'Infinite Scroll' }}
      />
      <Tab.Screen
        name="PullToRefresh"
        component={PullToRefresh}
        options={{ title: 'Pull to Refresh' }}
      />
      <Tab.Screen
        name="OfflineCaching"
        component={OfflineCaching}
        options={{ title: 'Offline Caching' }}
      />
      <Tab.Screen
        name="NetworkSwitchDialog"
        component={NetworkSwitchScreen}
        options={{ title: 'Network Switch' }}
      />
      <Tab.Screen
        name="SearchFunctionality"
        component={SearchFunctionalityScreen}
        options={{ title: 'Search' }}
      />
      <Tab.Screen
        name="IOSProjectSetup"
        component={IOSProjectSetup}
        options={{ title: 'iOS Setup' }}
      />
      <Tab.Screen name="Settings" component={SettingsScreen} />
    </Tab.Navigator>
  );
}

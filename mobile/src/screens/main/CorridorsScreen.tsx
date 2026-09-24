import React from 'react';
import {
  FlatList,
  Pressable,
  StyleSheet,
  Text,
  View,
} from 'react-native';
import { NativeStackNavigationProp } from '@react-navigation/native-stack';
import { useNavigation } from '@react-navigation/native';

import { CorridorsStackParamList } from '@navigation/MainNavigator';

const SAMPLE_CORRIDORS = [
  { id: 'USDC-PHP', label: 'USDC → PHP' },
  { id: 'USDC-JPY', label: 'USDC → JPY' },
  { id: 'EURC-NGN', label: 'EURC → NGN' },
];

type CorridorsScreenNavigationProp = NativeStackNavigationProp<
  CorridorsStackParamList,
  'CorridorsList'
>;

export function CorridorsScreen() {
  const navigation = useNavigation<CorridorsScreenNavigationProp>();

  return (
    <View style={styles.container}>
      <FlatList
        data={SAMPLE_CORRIDORS}
        keyExtractor={item => item.id}
        contentContainerStyle={styles.listContent}
        renderItem={({ item }) => (
          <Pressable
            style={styles.listItem}
            onPress={() =>
              navigation.navigate('CorridorDetail', { corridorId: item.id })
            }
            accessibilityRole="button"
            accessibilityLabel={`Open corridor detail for ${item.label}`}
          >
            <Text style={styles.listItemTitle}>{item.label}</Text>
            <Text style={styles.listItemSubtitle}>{item.id}</Text>
          </Pressable>
        )}
        ListEmptyComponent={
          <View style={styles.empty}>
            <Text style={styles.emptyText}>No corridors available</Text>
          </View>
        }
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  listContent: {
    padding: 16,
  },
  listItem: {
    minHeight: 44,
    backgroundColor: '#FFFFFF',
    borderRadius: 8,
    paddingHorizontal: 16,
    paddingVertical: 12,
    marginBottom: 12,
    borderWidth: 1,
    borderColor: '#E5E7EB',
  },
  listItemTitle: {
    fontSize: 16,
    fontWeight: '600',
    color: '#111827',
  },
  listItemSubtitle: {
    marginTop: 4,
    fontSize: 13,
    color: '#6B7280',
  },
  empty: {
    flex: 1,
    justifyContent: 'center',
    alignItems: 'center',
    padding: 40,
  },
  emptyText: {
    fontSize: 16,
    color: '#999',
  },
});

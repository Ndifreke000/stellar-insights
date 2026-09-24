import React from 'react';
import { View, Text, StyleSheet, FlatList } from 'react-native';

export function AnchorsScreen() {
  return (
    <View style={styles.container}>
      <FlatList
        data={[]}
        renderItem={() => null}
        ListEmptyComponent={
          <View style={styles.empty}>
            <Text style={styles.emptyText}>No anchors available</Text>
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

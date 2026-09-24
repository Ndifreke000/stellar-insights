import React from 'react';
import { ActivityIndicator, FlatList, Platform, Pressable, StyleSheet, Text, View } from 'react-native';
import { InfiniteScrollItem, useInfiniteScroll } from '@hooks/useInfiniteScroll';

export const InfiniteScroll: React.FC = () => {
  const { items, isLoading, hasMore, error, platformThreshold, loadMore, refresh } = useInfiniteScroll();

  const renderItem = React.useCallback(({ item }: { item: InfiniteScrollItem }) => (
    <View style={styles.card} accessible accessibilityLabel={`${item.title}. ${item.description}`}>
      <Text style={styles.cardTitle}>{item.title}</Text>
      <Text style={styles.cardDescription}>{item.description}</Text>
    </View>
  ), []);

  return (
    <View style={styles.container} accessibilityLabel="Infinite scroll screen">
      <View style={styles.header}>
        <Text style={styles.title}>Infinite Scroll</Text>
        <Text style={styles.subtitle}>Pull to refresh or scroll to load more insights</Text>
      </View>

      {error ? (
        <View style={styles.errorCard} accessibilityRole="alert" accessibilityLabel={`Infinite scroll error ${error}`}>
          <Text style={styles.errorText}>{error}</Text>
        </View>
      ) : null}

      <FlatList
        data={items}
        keyExtractor={item => item.id}
        renderItem={renderItem}
        onEndReached={loadMore}
        onEndReachedThreshold={platformThreshold}
        refreshing={isLoading && items.length <= 12}
        onRefresh={refresh}
        contentContainerStyle={styles.listContent}
        accessibilityLabel="Infinite scroll list"
        ListFooterComponent={
          <View style={styles.footer}>
            {isLoading ? <ActivityIndicator accessibilityLabel="Loading more results" /> : null}
            {!hasMore ? <Text style={styles.footerText}>No more results</Text> : null}
            {hasMore && !isLoading ? (
              <Pressable onPress={loadMore} style={styles.loadMoreButton} accessibilityRole="button" accessibilityLabel="Load more infinite scroll results">
                <Text style={styles.loadMoreText}>Load more</Text>
              </Pressable>
            ) : null}
          </View>
        }
      />
    </View>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  header: {
    padding: 20,
    backgroundColor: '#fff',
  },
  title: {
    fontSize: 28,
    fontWeight: 'bold',
  },
  subtitle: {
    color: '#666',
    marginTop: 6,
  },
  errorCard: {
    backgroundColor: '#fee2e2',
    margin: 16,
    padding: 14,
    borderRadius: 12,
  },
  errorText: {
    color: '#991b1b',
  },
  listContent: {
    padding: 16,
    paddingBottom: Platform.select({ ios: 32, android: 24, default: 24 }),
  },
  card: {
    backgroundColor: '#fff',
    borderRadius: Platform.OS === 'ios' ? 14 : 10,
    padding: 16,
    marginBottom: 12,
    shadowColor: '#000',
    shadowOffset: { width: 0, height: 2 },
    shadowOpacity: Platform.OS === 'ios' ? 0.1 : 0,
    shadowRadius: 4,
    elevation: Platform.OS === 'android' ? 2 : 0,
  },
  cardTitle: {
    fontSize: 17,
    fontWeight: '700',
  },
  cardDescription: {
    color: '#666',
    marginTop: 6,
  },
  footer: {
    minHeight: 64,
    alignItems: 'center',
    justifyContent: 'center',
  },
  footerText: {
    color: '#666',
  },
  loadMoreButton: {
    minHeight: 44,
    justifyContent: 'center',
    paddingHorizontal: 16,
  },
  loadMoreText: {
    color: '#2563eb',
    fontWeight: '700',
  },
});

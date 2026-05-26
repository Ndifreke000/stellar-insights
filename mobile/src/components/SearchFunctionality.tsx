import React from 'react';
import {
  ActivityIndicator,
  FlatList,
  Platform,
  Pressable,
  StyleSheet,
  Text,
  TextInput,
  View,
} from 'react-native';
import { SearchResult, useSearchFunctionality } from '@hooks/useSearchFunctionality';

function useDebouncedValue(value: string, delay: number) {
  const [debouncedValue, setDebouncedValue] = React.useState(value);

  React.useEffect(() => {
    const timeout = setTimeout(() => setDebouncedValue(value), delay);

    return () => clearTimeout(timeout);
  }, [delay, value]);

  return debouncedValue;
}

export function SearchFunctionality() {
  const [query, setQuery] = React.useState('');
  const debouncedQuery = useDebouncedValue(query, 250);
  const { results, canSearch, isOnline, isPending, isFetching, isError, refetch } =
    useSearchFunctionality(debouncedQuery);

  const renderResult = ({ item }: { item: SearchResult }) => (
    <View
      accessibilityLabel={`${item.type}: ${item.title}`}
      style={styles.result}
      testID={`search-result-${item.id}`}>
      <Text style={styles.resultType}>{item.type}</Text>
      <Text style={styles.resultTitle}>{item.title}</Text>
      <Text style={styles.resultSubtitle} numberOfLines={1}>
        {item.subtitle}
      </Text>
    </View>
  );

  const renderEmptyState = () => {
    if (!query.trim()) {
      return <Text style={styles.message}>Search anchors, assets, or payment corridors.</Text>;
    }

    if (query.trim().length < 2) {
      return <Text style={styles.message}>Enter at least two characters to search.</Text>;
    }

    if (!isOnline && results.length === 0) {
      return (
        <Text style={styles.message}>Search requires a connection until results are cached.</Text>
      );
    }

    if (isPending || isFetching) {
      return <ActivityIndicator accessibilityLabel="Searching" color="#007AFF" size="large" />;
    }

    if (isError) {
      return (
        <View style={styles.feedback}>
          <Text style={styles.message}>Search could not be loaded. Please try again.</Text>
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Retry search"
            onPress={() => refetch()}
            style={styles.retryButton}>
            <Text style={styles.retryText}>Retry</Text>
          </Pressable>
        </View>
      );
    }

    if (canSearch) {
      return <Text style={styles.message}>No matching anchors or corridors found.</Text>;
    }

    return null;
  };

  return (
    <View style={styles.container}>
      <View style={styles.searchRow}>
        <TextInput
          accessibilityLabel="Search anchors and corridors"
          autoCapitalize="none"
          autoCorrect={false}
          clearButtonMode={Platform.OS === 'ios' ? 'while-editing' : 'never'}
          keyboardAppearance="default"
          onChangeText={setQuery}
          placeholder="Search anchors, assets, corridors"
          returnKeyType="search"
          style={styles.input}
          value={query}
        />
        {Platform.OS === 'android' && query.length > 0 ? (
          <Pressable
            accessibilityRole="button"
            accessibilityLabel="Clear search"
            onPress={() => setQuery('')}
            style={styles.clearButton}>
            <Text style={styles.clearText}>Clear</Text>
          </Pressable>
        ) : null}
      </View>

      {!isOnline && results.length > 0 ? (
        <Text accessibilityRole="alert" style={styles.offline}>
          Offline: showing cached search results.
        </Text>
      ) : null}

      <FlatList
        contentContainerStyle={results.length === 0 ? styles.emptyList : styles.list}
        data={results}
        keyboardShouldPersistTaps="handled"
        keyExtractor={item => `${item.type}-${item.id}`}
        ListEmptyComponent={renderEmptyState}
        renderItem={renderResult}
      />
    </View>
  );
}

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#f5f5f5',
  },
  searchRow: {
    alignItems: 'center',
    backgroundColor: '#fff',
    flexDirection: 'row',
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
  input: {
    backgroundColor: '#f0f2f5',
    borderRadius: 10,
    flex: 1,
    fontSize: 16,
    height: 44,
    paddingHorizontal: 14,
  },
  clearButton: {
    marginLeft: 10,
    paddingVertical: 10,
  },
  clearText: {
    color: '#007AFF',
    fontSize: 15,
    fontWeight: '500',
  },
  offline: {
    backgroundColor: '#fff4d6',
    color: '#765800',
    paddingHorizontal: 16,
    paddingVertical: 10,
  },
  emptyList: {
    alignItems: 'center',
    flexGrow: 1,
    justifyContent: 'center',
    paddingHorizontal: 32,
  },
  list: {
    paddingVertical: 8,
  },
  message: {
    color: '#666',
    fontSize: 16,
    textAlign: 'center',
  },
  feedback: {
    alignItems: 'center',
  },
  retryButton: {
    backgroundColor: '#007AFF',
    borderRadius: 8,
    marginTop: 16,
    paddingHorizontal: 20,
    paddingVertical: 10,
  },
  retryText: {
    color: '#fff',
    fontSize: 15,
    fontWeight: '600',
  },
  result: {
    backgroundColor: '#fff',
    borderBottomColor: '#eee',
    borderBottomWidth: 1,
    paddingHorizontal: 20,
    paddingVertical: 14,
  },
  resultType: {
    color: '#007AFF',
    fontSize: 12,
    fontWeight: '700',
    marginBottom: 5,
    textTransform: 'uppercase',
  },
  resultTitle: {
    color: '#111',
    fontSize: 17,
    fontWeight: '600',
  },
  resultSubtitle: {
    color: '#666',
    fontSize: 14,
    marginTop: 5,
  },
});

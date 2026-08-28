import React from 'react';
import {
  ActivityIndicator,
  FlatList,
  Platform,
  Pressable,
  ScrollView,
  StyleSheet,
  TextInput,
  Text,
  View,
  ViewStyle,
} from 'react-native';
import {
  useSearchFunctionality,
  SearchableItem,
  SearchResult,
  SearchConfig,
} from '@hooks/useSearchFunctionality';

export interface SearchFunctionalityProps {
  data: SearchableItem[];
  renderItem: (item: { item: SearchableItem; score: number }) => React.ReactElement | null;
  onItemPress?: (item: SearchableItem) => void;
  placeholder?: string;
  config?: SearchConfig;
  showFilters?: boolean;
  contentContainerStyle?: ViewStyle;
  testID?: string;
}

/**
 * Search Functionality component
 * Provides search UI with filtering, pagination, and result display
 */
export const SearchFunctionality: React.FC<SearchFunctionalityProps> = ({
  data,
  renderItem,
  onItemPress,
  placeholder = 'Search...',
  config,
  showFilters = true,
  contentContainerStyle,
  testID,
}) => {
  const {
    query,
    results,
    isSearching,
    error,
    totalResults,
    hasMore,
    clearSearch,
    setQuery,
    loadMore,
  } = useSearchFunctionality(data, config);

  const handleClearSearch = React.useCallback(() => {
    clearSearch();
  }, [clearSearch]);

  return (
    <View style={styles.container} testID={testID}>
      {/* Search Bar */}
      <View style={styles.searchBarContainer}>
        <View style={styles.searchBar}>
          <Text style={styles.searchIcon}>🔍</Text>
          <TextInput
            style={styles.searchInput}
            placeholder={placeholder}
            placeholderTextColor="#999999"
            value={query}
            onChangeText={setQuery}
            editable={!isSearching}
            accessibilityLabel="Search input"
            testID="search-input"
          />
          {query.length > 0 && (
            <Pressable
              onPress={handleClearSearch}
              accessibilityRole="button"
              accessibilityLabel="Clear search">
              <Text style={styles.clearButton}>✕</Text>
            </Pressable>
          )}
          {isSearching && <ActivityIndicator size="small" color="#0066CC" />}
        </View>
      </View>

      {/* Error Display */}
      {error && (
        <View style={styles.errorContainer} accessibilityRole="alert">
          <Text style={styles.errorIcon}>⚠️</Text>
          <Text style={styles.errorText}>{error}</Text>
        </View>
      )}

      {/* Results Information */}
      {query.length > 0 && !isSearching && (
        <View style={styles.resultsInfo}>
          <Text style={styles.resultsText}>
            {totalResults === 0
              ? 'No results found'
              : `Found ${totalResults} result${totalResults !== 1 ? 's' : ''}`}
          </Text>
        </View>
      )}

      {/* Results List */}
      {query.length > 0 ? (
        <FlatList
          data={results}
          renderItem={({ item }) => (
            <Pressable
              style={({ pressed }) => [
                styles.resultItem,
                { backgroundColor: pressed ? '#F5F5F5' : '#FFFFFF' },
              ]}
              onPress={() => onItemPress?.(item.item)}
              accessibilityRole="button"
              accessibilityLabel={`Result: ${JSON.stringify(item.item).substring(0, 50)}`}>
              {renderItem({ item: item.item, score: item.score })}
              {item.score > 0 && (
                <View style={styles.scoreContainer}>
                  <Text style={styles.scoreText}>{(item.score * 100).toFixed(0)}% match</Text>
                </View>
              )}
            </Pressable>
          )}
          keyExtractor={(item, index) => `${index}-${item.score}`}
          contentContainerStyle={[styles.listContent, contentContainerStyle]}
          ListEmptyComponent={
            query.length > 0 ? (
              <View style={styles.emptyContainer} accessibilityLiveRegion="polite">
                <Text style={styles.emptyIcon}>🔎</Text>
                <Text style={styles.emptyText}>No results found</Text>
                <Text style={styles.emptySubtext}>Try different search terms</Text>
              </View>
            ) : null
          }
          ListFooterComponent={
            hasMore ? (
              <Pressable
                style={({ pressed }) => [styles.loadMoreButton, { opacity: pressed ? 0.7 : 1 }]}
                onPress={loadMore}
                accessibilityRole="button"
                accessibilityLabel="Load more results">
                <Text style={styles.loadMoreText}>Load More Results</Text>
              </Pressable>
            ) : null
          }
          accessibilityLabel="Search results list"
        />
      ) : (
        <View style={styles.emptySearchContainer}>
          <Text style={styles.emptySearchIcon}>🔍</Text>
          <Text style={styles.emptySearchTitle}>Start Searching</Text>
          <Text style={styles.emptySearchText}>Enter a search term to find results</Text>
        </View>
      )}
    </View>
  );
};

/**
 * Quick Search Bar component
 * Minimal search input for embedding in headers or other layouts
 */
export interface QuickSearchBarProps {
  onSearch: (query: string) => void;
  onClear?: () => void;
  placeholder?: string;
  testID?: string;
}

export const QuickSearchBar: React.FC<QuickSearchBarProps> = ({
  onSearch,
  onClear,
  placeholder = 'Search...',
  testID,
}) => {
  const [query, setQuery] = React.useState('');

  const handleChangeText = React.useCallback(
    (text: string) => {
      setQuery(text);
      onSearch(text);
    },
    [onSearch]
  );

  const handleClear = React.useCallback(() => {
    setQuery('');
    onClear?.();
  }, [onClear]);

  return (
    <View style={styles.quickSearchBar} testID={testID}>
      <Text style={styles.searchIcon}>🔍</Text>
      <TextInput
        style={styles.quickSearchInput}
        placeholder={placeholder}
        placeholderTextColor="#999999"
        value={query}
        onChangeText={handleChangeText}
        accessibilityLabel="Quick search input"
      />
      {query.length > 0 && (
        <Pressable
          onPress={handleClear}
          accessibilityRole="button"
          accessibilityLabel="Clear search">
          <Text style={styles.clearButton}>✕</Text>
        </Pressable>
      )}
    </View>
  );
};

/**
 * Search Result Item component
 * Reusable component for displaying individual search results
 */
export interface SearchResultItemProps {
  title: string;
  subtitle?: string;
  description?: string;
  score?: number;
  matchedFields?: string[];
  onPress?: () => void;
  testID?: string;
}

export const SearchResultItem: React.FC<SearchResultItemProps> = ({
  title,
  subtitle,
  description,
  score,
  matchedFields,
  onPress,
  testID,
}) => {
  return (
    <Pressable
      style={({ pressed }) => [
        styles.searchResultItem,
        { backgroundColor: pressed ? '#F5F5F5' : '#FFFFFF' },
      ]}
      onPress={onPress}
      accessibilityRole="button"
      accessibilityLabel={title}
      testID={testID}>
      <View style={styles.resultContent}>
        <Text style={styles.resultTitle}>{title}</Text>
        {subtitle && <Text style={styles.resultSubtitle}>{subtitle}</Text>}
        {description && <Text style={styles.resultDescription}>{description}</Text>}
        {matchedFields && matchedFields.length > 0 && (
          <View style={styles.matchedFieldsContainer}>
            {matchedFields.slice(0, 2).map((field, index) => (
              <View key={index} style={styles.fieldBadge}>
                <Text style={styles.fieldBadgeText}>{field}</Text>
              </View>
            ))}
            {matchedFields.length > 2 && (
              <Text style={styles.moreFieldsText}>+{matchedFields.length - 2} more</Text>
            )}
          </View>
        )}
      </View>
      {score !== undefined && (
        <View style={styles.scoreIndicator}>
          <Text style={styles.scoreIndicatorText}>{(score * 100).toFixed(0)}%</Text>
        </View>
      )}
    </Pressable>
  );
};

const styles = StyleSheet.create({
  container: {
    flex: 1,
    backgroundColor: '#FAFAFA',
  },
  searchBarContainer: {
    paddingHorizontal: 16,
    paddingVertical: 12,
    backgroundColor: '#FFFFFF',
    borderBottomWidth: 1,
    borderBottomColor: '#E0E0E0',
  },
  searchBar: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: '#F5F5F5',
    borderRadius: 8,
    paddingHorizontal: 12,
    height: 44,
    gap: 8,
  },
  searchIcon: {
    fontSize: 18,
  },
  searchInput: {
    flex: 1,
    fontSize: 16,
    color: '#212121',
    paddingVertical: 8,
  },
  clearButton: {
    fontSize: 18,
    color: '#999999',
    padding: 4,
  },
  errorContainer: {
    marginHorizontal: 16,
    marginTop: 12,
    paddingHorizontal: 12,
    paddingVertical: 10,
    backgroundColor: '#FFEBEE',
    borderRadius: 8,
    borderLeftWidth: 4,
    borderLeftColor: '#D32F2F',
    flexDirection: 'row',
    alignItems: 'center',
    gap: 8,
  },
  errorIcon: {
    fontSize: 18,
  },
  errorText: {
    flex: 1,
    color: '#C62828',
    fontWeight: '500',
    fontSize: 13,
  },
  resultsInfo: {
    paddingHorizontal: 16,
    paddingVertical: 12,
    backgroundColor: '#FFFFFF',
    borderBottomWidth: 1,
    borderBottomColor: '#E0E0E0',
  },
  resultsText: {
    fontSize: 14,
    color: '#666666',
    fontWeight: '500',
  },
  listContent: {
    paddingHorizontal: 16,
    paddingVertical: 12,
  },
  resultItem: {
    marginBottom: 8,
    borderRadius: 8,
    padding: 12,
    borderWidth: 1,
    borderColor: '#E0E0E0',
    flexDirection: 'row',
    alignItems: 'center',
    gap: 12,
  },
  scoreContainer: {
    backgroundColor: '#E3F2FD',
    borderRadius: 6,
    paddingHorizontal: 8,
    paddingVertical: 4,
  },
  scoreText: {
    fontSize: 11,
    fontWeight: '600',
    color: '#0066CC',
  },
  emptyContainer: {
    alignItems: 'center',
    justifyContent: 'center',
    paddingVertical: 60,
  },
  emptyIcon: {
    fontSize: 48,
    marginBottom: 16,
  },
  emptyText: {
    fontSize: 16,
    fontWeight: '600',
    color: '#212121',
    marginBottom: 4,
  },
  emptySubtext: {
    fontSize: 13,
    color: '#999999',
  },
  loadMoreButton: {
    marginTop: 12,
    marginBottom: 12,
    paddingVertical: 12,
    paddingHorizontal: 16,
    backgroundColor: '#E3F2FD',
    borderRadius: 8,
    alignItems: 'center',
  },
  loadMoreText: {
    fontSize: 14,
    fontWeight: '600',
    color: '#0066CC',
  },
  emptySearchContainer: {
    flex: 1,
    alignItems: 'center',
    justifyContent: 'center',
    paddingHorizontal: 32,
  },
  emptySearchIcon: {
    fontSize: 64,
    marginBottom: 16,
  },
  emptySearchTitle: {
    fontSize: 18,
    fontWeight: '700',
    color: '#212121',
    marginBottom: 8,
    textAlign: 'center',
  },
  emptySearchText: {
    fontSize: 14,
    color: '#999999',
    textAlign: 'center',
  },
  // Quick search bar
  quickSearchBar: {
    flexDirection: 'row',
    alignItems: 'center',
    backgroundColor: '#F5F5F5',
    borderRadius: 8,
    paddingHorizontal: 12,
    height: 40,
    gap: 8,
    marginHorizontal: 16,
    marginVertical: 8,
  },
  quickSearchInput: {
    flex: 1,
    fontSize: 14,
    color: '#212121',
    paddingVertical: 6,
  },
  // Search result item
  searchResultItem: {
    marginBottom: 8,
    borderRadius: 8,
    padding: 12,
    borderWidth: 1,
    borderColor: '#E0E0E0',
    flexDirection: 'row',
    alignItems: 'center',
    justifyContent: 'space-between',
  },
  resultContent: {
    flex: 1,
    gap: 4,
  },
  resultTitle: {
    fontSize: 15,
    fontWeight: '600',
    color: '#212121',
  },
  resultSubtitle: {
    fontSize: 12,
    color: '#666666',
    fontWeight: '500',
  },
  resultDescription: {
    fontSize: 12,
    color: '#999999',
    marginTop: 2,
  },
  matchedFieldsContainer: {
    flexDirection: 'row',
    flexWrap: 'wrap',
    gap: 6,
    marginTop: 6,
  },
  fieldBadge: {
    backgroundColor: '#F0F4FF',
    borderRadius: 4,
    paddingHorizontal: 6,
    paddingVertical: 2,
  },
  fieldBadgeText: {
    fontSize: 10,
    color: '#0066CC',
    fontWeight: '500',
  },
  moreFieldsText: {
    fontSize: 10,
    color: '#999999',
    fontWeight: '500',
  },
  scoreIndicator: {
    width: 44,
    height: 44,
    borderRadius: 22,
    backgroundColor: '#E3F2FD',
    justifyContent: 'center',
    alignItems: 'center',
  },
  scoreIndicatorText: {
    fontSize: 12,
    fontWeight: '700',
    color: '#0066CC',
  },
});

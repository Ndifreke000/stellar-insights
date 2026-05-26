jest.mock('@services/api', () => ({
  apiClient: { get: jest.fn() },
}));

import { filterSearchResults, SearchResult } from '../useSearchFunctionality';

const results: SearchResult[] = [
  {
    id: 'anchor-1',
    type: 'anchor',
    title: 'Circle',
    subtitle: 'active | GCIRCLE',
    searchText: 'Circle GCIRCLE active',
  },
  {
    id: 'corridor-1',
    type: 'corridor',
    title: 'USDC to XLM',
    subtitle: '99.0% success rate',
    searchText: 'USDC XLM',
  },
];

describe('filterSearchResults', () => {
  it('matches anchors without case sensitivity', () => {
    expect(filterSearchResults(results, ' circle ')).toEqual([results[0]]);
  });

  it('matches corridor assets', () => {
    expect(filterSearchResults(results, 'xlm')).toEqual([results[1]]);
  });

  it('does not return results for blank searches', () => {
    expect(filterSearchResults(results, '   ')).toEqual([]);
  });

  it('limits large result sets for mobile rendering', () => {
    const largeResults = Array.from({ length: 35 }, (_, index) => ({
      ...results[0],
      id: String(index),
    }));

    expect(filterSearchResults(largeResults, 'circle')).toHaveLength(30);
  });
});

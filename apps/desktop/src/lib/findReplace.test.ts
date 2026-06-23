import { describe, expect, it } from 'vitest';
import { findTextRanges } from './findReplace';

describe('findTextRanges', () => {
  it('finds non-overlapping matches case-insensitively by default', () => {
    expect(findTextRanges('Alpha beta alpha', 'alpha')).toEqual([
      { index: 0, length: 5 },
      { index: 11, length: 5 }
    ]);
  });

  it('supports case-sensitive matching', () => {
    expect(findTextRanges('Alpha alpha', 'Alpha', true)).toEqual([{ index: 0, length: 5 }]);
  });

  it('returns no ranges for empty queries', () => {
    expect(findTextRanges('Draft', '')).toEqual([]);
  });
});

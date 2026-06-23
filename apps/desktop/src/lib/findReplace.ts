export interface FindRange {
  index: number;
  length: number;
}

export function findTextRanges(text: string, query: string, caseSensitive = false): FindRange[] {
  if (query.length === 0) {
    return [];
  }

  const haystack = caseSensitive ? text : text.toLocaleLowerCase();
  const needle = caseSensitive ? query : query.toLocaleLowerCase();
  const ranges: FindRange[] = [];
  let offset = 0;

  while (offset <= haystack.length) {
    const index = haystack.indexOf(needle, offset);
    if (index === -1) {
      break;
    }
    ranges.push({ index, length: query.length });
    offset = index + Math.max(needle.length, 1);
  }

  return ranges;
}

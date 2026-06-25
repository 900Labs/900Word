import { type AssetRef, type DocumentState } from './documentProjection';
import {
  buildExpandedDocumentStats,
  type CoreDocumentStats
} from './documentStats';

export type DocumentInspectorSavedStatus = 'saved' | 'saved_with_unsaved_changes' | 'unsaved';
export type DocumentInspectorLocationStatus = 'backend_only' | 'none';
export type DocumentInspectorPrivacyWarningKind =
  | 'comments'
  | 'tracked_changes'
  | 'metadata'
  | 'recovery'
  | 'unsaved';

export interface DocumentInspectorFileState {
  has_current_path: boolean;
  dirty: boolean;
  recovery_documents?: unknown[];
}

export interface DocumentInspectorInput {
  coreStats: CoreDocumentStats;
  document: DocumentState | undefined;
  fileState: DocumentInspectorFileState;
  plainText: string;
  selectionWordCount: number;
}

export interface DocumentInspectorSummary {
  format: 'OpenDocument Text (.odt)';
  savedStatus: DocumentInspectorSavedStatus;
  locationStatus: DocumentInspectorLocationStatus;
  createdAt: string;
  modifiedAt: string;
  pageSize: string;
  wordCount: number;
  characterCount: number;
  characterCountWithoutSpaces: number;
  paragraphCount: number;
  blockCount: number;
  estimatedPageCount: number;
  selectionWordCount: number;
  embeddedImageCount: number;
  embeddedImageBytes: number;
  embeddedImageBytesLabel: string;
  commentCount: number;
  unresolvedCommentCount: number;
  trackChangesRecording: boolean;
  trackedChangeCount: number;
  footnoteCount: number;
  endnoteCount: number;
  privacyWarnings: DocumentInspectorPrivacyWarningKind[];
}

export function buildDocumentInspectorSummary(input: DocumentInspectorInput): DocumentInspectorSummary {
  const expandedStats = buildExpandedDocumentStats({
    coreStats: input.coreStats,
    document: input.document,
    plainText: input.plainText,
    selectionWordCount: input.selectionWordCount,
    pageSetup: input.document?.sections[0]?.page
  });
  const imageSummary = summarizeEmbeddedImageAssets(input.document);
  const savedStatus = documentSavedStatus(input.fileState);
  const privacyWarnings = documentPrivacyWarnings({
    commentCount: expandedStats.commentCount,
    trackChangesRecording: expandedStats.trackChangesRecording,
    trackedChangeCount: expandedStats.trackedChangeCount,
    hasMetadata: Boolean(input.document?.meta?.title),
    hasRecoveryDrafts: (input.fileState.recovery_documents?.length ?? 0) > 0,
    savedStatus
  });

  return {
    format: 'OpenDocument Text (.odt)',
    savedStatus,
    locationStatus: input.fileState.has_current_path ? 'backend_only' : 'none',
    createdAt: safeTimestamp(input.document?.meta?.created_at),
    modifiedAt: safeTimestamp(input.document?.meta?.modified_at),
    pageSize: expandedStats.pageSize,
    wordCount: expandedStats.wordCount,
    characterCount: expandedStats.characterCountWithSpaces,
    characterCountWithoutSpaces: expandedStats.characterCountWithoutSpaces,
    paragraphCount: expandedStats.paragraphCount,
    blockCount: expandedStats.blockCount,
    estimatedPageCount: expandedStats.estimatedPageCount,
    selectionWordCount: expandedStats.selectionWordCount,
    embeddedImageCount: imageSummary.count,
    embeddedImageBytes: imageSummary.bytes,
    embeddedImageBytesLabel: formatBytes(imageSummary.bytes),
    commentCount: expandedStats.commentCount,
    unresolvedCommentCount: expandedStats.unresolvedCommentCount,
    trackChangesRecording: expandedStats.trackChangesRecording,
    trackedChangeCount: expandedStats.trackedChangeCount,
    footnoteCount: expandedStats.footnoteCount,
    endnoteCount: expandedStats.endnoteCount,
    privacyWarnings
  };
}

function safeTimestamp(value: unknown): string {
  return typeof value === 'string' && value.trim().length > 0 ? value : '';
}

export function documentSavedStatus(fileState: DocumentInspectorFileState): DocumentInspectorSavedStatus {
  if (!fileState.has_current_path) {
    return 'unsaved';
  }
  return fileState.dirty ? 'saved_with_unsaved_changes' : 'saved';
}

export function formatBytes(value: number): string {
  const normalized = Math.max(0, Math.trunc(Number.isFinite(value) ? value : 0));
  if (normalized < 1024) {
    return `${normalized} B`;
  }
  if (normalized < 1024 * 1024) {
    return `${formatByteUnit(normalized / 1024)} KB`;
  }
  return `${formatByteUnit(normalized / (1024 * 1024))} MB`;
}

function formatByteUnit(value: number): string {
  if (value >= 10 || Number.isInteger(value)) {
    return String(Math.round(value));
  }
  const fixed = value.toFixed(1);
  return fixed.endsWith('.0') ? String(Math.round(value)) : fixed;
}

function summarizeEmbeddedImageAssets(document: DocumentState | undefined): { count: number; bytes: number } {
  const assets = Object.values(document?.assets ?? {}).filter(isImageAsset);
  return {
    count: assets.length,
    bytes: assets.reduce((total, asset) => total + assetByteLength(asset), 0)
  };
}

function isImageAsset(asset: AssetRef): boolean {
  return asset.media_type.toLocaleLowerCase().startsWith('image/');
}

function assetByteLength(asset: AssetRef): number {
  if (Number.isFinite(asset.byte_len) && asset.byte_len >= 0) {
    return Math.trunc(asset.byte_len);
  }
  return Array.isArray(asset.bytes) ? asset.bytes.length : 0;
}

function documentPrivacyWarnings(input: {
  commentCount: number;
  trackChangesRecording: boolean;
  trackedChangeCount: number;
  hasMetadata: boolean;
  hasRecoveryDrafts: boolean;
  savedStatus: DocumentInspectorSavedStatus;
}): DocumentInspectorPrivacyWarningKind[] {
  const warnings: DocumentInspectorPrivacyWarningKind[] = [];
  if (input.commentCount > 0) {
    warnings.push('comments');
  }
  if (input.trackChangesRecording || input.trackedChangeCount > 0) {
    warnings.push('tracked_changes');
  }
  if (input.hasMetadata) {
    warnings.push('metadata');
  }
  if (input.hasRecoveryDrafts) {
    warnings.push('recovery');
  }
  if (input.savedStatus !== 'saved') {
    warnings.push('unsaved');
  }
  return warnings;
}
